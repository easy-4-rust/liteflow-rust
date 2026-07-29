//! Redis keyspace 订阅模式。

use std::time::Duration;

use futures_util::StreamExt;
use futures_util::stream::select_all;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::RuleSourceWatcher;

use crate::parser::redis::vo::RedisParserVO;
use crate::parser::redis::{RedisSubscribeHandle, RedisXmlELParser};

use super::super::{RClient, RedisParserHelper};

/// 通过 Redis keyspace notification 监听规则 key 的变化。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.redis.mode.subscribe.RedisParserSubscribeMode`。
/// Java 使用 Redisson `RMap` 条目监听器；Rust 订阅同一 Redis 数据库的 keyspace
/// 事件，并在收到 `set`、`hset`、`hdel`、`del` 等事件后执行完整规则对账。
pub struct RedisParserSubscribeMode {
    parser: Option<RedisXmlELParser>,
    subscriptions: Vec<(RClient, String)>,
}

impl RedisParserSubscribeMode {
    /// 创建 Chain 规则 key 的订阅模式。
    ///
    /// 参数 `url` 对应 Java Redis 连接配置，`chain_key` 对应
    /// `RedisParserVO#getChainKey`。
    #[must_use]
    pub fn new(url: impl Into<String>, chain_key: impl Into<String>) -> Self {
        Self {
            parser: None,
            subscriptions: vec![(RClient::new(url), chain_key.into())],
        }
    }

    /// 按 Java 扩展配置创建单点、Sentinel 或 Cluster 订阅模式。
    ///
    /// Chain 和 Script 可以位于不同逻辑数据库；Cluster 会在监听开始时发现全部
    /// 健康主节点。对应 Java `RedisParserSubscribeMode#RedisParserSubscribeMode`。
    pub fn from_config(config: RedisParserVO) -> LFResult<Self> {
        config.validate().map_err(LiteflowError::Rule)?;
        let parser = RedisXmlELParser::new(config.clone())?;
        let chain_key = config
            .chain_key
            .clone()
            .ok_or_else(|| LiteflowError::Rule("redis chainKey is blank".to_string()))?;
        let chain_client =
            <Self as RedisParserHelper>::get_redis_client(&config, config.chain_data_base)?
                .ok_or_else(|| LiteflowError::Rule("redis chainDataBase is blank".to_string()))?;
        let mut subscriptions = vec![(chain_client, chain_key)];
        if let Some(script_key) = config.script_key.clone() {
            let script_client =
                <Self as RedisParserHelper>::get_redis_client(&config, config.script_data_base)?
                    .ok_or_else(|| {
                        LiteflowError::Rule("redis scriptDataBase is blank".to_string())
                    })?;
            subscriptions.push((script_client, script_key));
        }
        Ok(Self {
            parser: Some(parser),
            subscriptions,
        })
    }

    /// 追加脚本规则 key。
    ///
    /// 参数 `script_key` 对应 Java `RedisParserVO#getScriptKey`。
    #[must_use]
    pub fn with_script_key(mut self, script_key: impl Into<String>) -> Self {
        let script_key = script_key.into();
        if !script_key.trim().is_empty()
            && !self.subscriptions.iter().any(|(_, key)| key == &script_key)
        {
            if let Some((client, _)) = self.subscriptions.first() {
                self.subscriptions.push((client.clone(), script_key));
            }
        }
        self
    }

    /// 校验订阅 key 是否完整，空白 Chain/Script key 会被拒绝。
    ///
    /// 对应 Java `RedisXmlELParser#checkParserVO` 的 `chainKey` 校验。
    pub fn validate(&self) -> LFResult<()> {
        if self.subscriptions.is_empty()
            || self
                .subscriptions
                .iter()
                .any(|(_, key)| key.trim().is_empty())
        {
            return Err(LiteflowError::Rule(
                "redis subscribe key must not be blank".to_string(),
            ));
        }
        Ok(())
    }

    /// 建立 Pub/Sub 连接并启动独立监听任务。
    ///
    /// Redis 必须启用 keyspace notification，例如
    /// `notify-keyspace-events=KEA`。连接或订阅失败会在启动阶段直接返回错误；
    /// 运行期间每个变更事件都会调用 `RuleSourceWatcher#reload`。
    pub async fn listen_redis(&self, watcher: RuleSourceWatcher) -> LFResult<RedisSubscribeHandle> {
        self.validate()?;

        let (stop_sender, stop_receiver) = tokio::sync::watch::channel(false);
        let mut listener_tasks = Vec::with_capacity(self.subscriptions.len());
        for (topology_client, key) in &self.subscriptions {
            let topology_client = topology_client.clone();
            let key = key.clone();
            let clients = resolve_clients(topology_client.clone()).await?;
            let signature = client_signature(&clients);
            let pubsubs = open_pubsubs(&clients, &key).await?;
            let watcher = watcher.clone();
            let stop_receiver = stop_receiver.clone();
            listener_tasks.push(tokio::spawn(subscription_worker(
                topology_client,
                key,
                watcher,
                stop_receiver,
                signature,
                pubsubs,
            )));
        }
        let task = tokio::spawn(async move {
            for listener_task in listener_tasks {
                let _ = listener_task.await;
            }
        });

        Ok(RedisSubscribeHandle::new(stop_sender, task))
    }
}

async fn subscription_worker(
    topology_client: RClient,
    key: String,
    watcher: RuleSourceWatcher,
    mut stop_receiver: tokio::sync::watch::Receiver<bool>,
    mut signature: Vec<String>,
    pubsubs: Vec<redis::aio::PubSub>,
) {
    let mut messages = select_all(pubsubs.into_iter().map(redis::aio::PubSub::into_on_message));
    let start = tokio::time::Instant::now() + Duration::from_secs(1);
    let mut topology_check = tokio::time::interval_at(start, Duration::from_secs(1));
    topology_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            changed = stop_receiver.changed() => {
                if changed.is_err() || *stop_receiver.borrow() {
                    break;
                }
            }
            _ = topology_check.tick() => {
                match resolve_clients(topology_client.clone()).await {
                    Ok(clients) => {
                        let current_signature = client_signature(&clients);
                        if current_signature != signature {
                            match open_pubsubs(&clients, &key).await {
                                Ok(pubsubs) => {
                                    // 先建立新拓扑订阅，再替换旧连接，缩小主从切换或扩缩容窗口。
                                    messages = select_all(
                                        pubsubs
                                            .into_iter()
                                            .map(redis::aio::PubSub::into_on_message),
                                    );
                                    signature = current_signature;
                                }
                                Err(error) => eprintln!(
                                    "[liteflow] redis topology refresh subscribe failed: {error}"
                                ),
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("[liteflow] redis topology refresh failed: {error}");
                    }
                }
            }
            message = messages.next() => {
                let Some(message) = message else {
                    // 所有连接断开后等待下一次拓扑检查，避免无连接状态下忙循环。
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    match resolve_clients(topology_client.clone()).await {
                        Ok(clients) => match open_pubsubs(&clients, &key).await {
                            Ok(pubsubs) => {
                                signature = client_signature(&clients);
                                messages = select_all(
                                    pubsubs
                                        .into_iter()
                                        .map(redis::aio::PubSub::into_on_message),
                                );
                            }
                            Err(error) => eprintln!(
                                "[liteflow] redis pubsub reconnect failed: {error}"
                            ),
                        },
                        Err(error) => {
                            eprintln!("[liteflow] redis topology reconnect failed: {error}");
                        }
                    }
                    continue;
                };
                let event = message
                    .get_payload::<String>()
                    .unwrap_or_else(|_| "unknown".to_string());
                reload_after_event(&watcher, &event).await;
            }
        }
    }
}

async fn reload_after_event(watcher: &RuleSourceWatcher, event: &str) {
    const MAX_RELOAD_ATTEMPTS: usize = 30;
    for attempt in 1..=MAX_RELOAD_ATTEMPTS {
        match watcher.reload().await {
            Ok(_) => return,
            Err(error) if attempt < MAX_RELOAD_ATTEMPTS => {
                eprintln!(
                    "[liteflow] redis keyspace event[{event}] reload attempt[{attempt}] failed: {error}"
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(error) => {
                eprintln!(
                    "[liteflow] redis keyspace event[{event}] reload failed after \
                     {MAX_RELOAD_ATTEMPTS} attempts: {error}"
                );
            }
        }
    }
}

async fn resolve_clients(topology_client: RClient) -> LFResult<Vec<redis::Client>> {
    tokio::task::spawn_blocking(move || topology_client.pubsub_clients())
        .await
        .map_err(|error| LiteflowError::Rule(format!("redis topology task error: {error}")))?
}

async fn open_pubsubs(clients: &[redis::Client], key: &str) -> LFResult<Vec<redis::aio::PubSub>> {
    let mut pubsubs = Vec::with_capacity(clients.len());
    for client in clients {
        let database = client.get_connection_info().redis_settings().db();
        let channel = keyspace_channel(database, key);
        let mut pubsub = client
            .get_async_pubsub()
            .await
            .map_err(|error| LiteflowError::Rule(format!("redis pubsub connect error: {error}")))?;
        pubsub.subscribe(&channel).await.map_err(|error| {
            LiteflowError::Rule(format!("redis subscribe channel[{channel}] error: {error}"))
        })?;
        pubsubs.push(pubsub);
    }
    Ok(pubsubs)
}

fn client_signature(clients: &[redis::Client]) -> Vec<String> {
    let mut signature = clients
        .iter()
        .map(|client| {
            let connection_info = client.get_connection_info();
            format!(
                "{:?}/{}",
                connection_info.addr(),
                connection_info.redis_settings().db()
            )
        })
        .collect::<Vec<_>>();
    signature.sort();
    signature
}

impl RedisParserHelper for RedisParserSubscribeMode {
    /// 聚合当前拓扑中的 Chain/Script Hash。
    ///
    /// 对应 Java `RedisParserSubscribeMode#getContent`。
    fn get_content(&self) -> LFResult<String> {
        self.parser
            .as_ref()
            .ok_or_else(|| {
                LiteflowError::Rule(
                    "legacy RedisParserSubscribeMode has no parser config".to_string(),
                )
            })?
            .get_content()
    }
}

fn keyspace_channel(database: i64, key: &str) -> String {
    format!("__keyspace@{database}__:{key}")
}

#[cfg(test)]
mod tests {
    use super::keyspace_channel;

    #[test]
    fn keyspace_channel_preserves_database_and_key() {
        assert_eq!(
            keyspace_channel(3, "liteflow:flow"),
            "__keyspace@3__:liteflow:flow"
        );
    }
}
