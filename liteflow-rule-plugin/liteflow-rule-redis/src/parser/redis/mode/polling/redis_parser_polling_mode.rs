//! Redis 定时轮询模式。

use std::time::Duration;

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, RuleSourceWatcher, fnv_fp};

use crate::parser::redis::RedisXmlELParser;
use crate::parser::redis::mode::RedisParserHelper;
use crate::parser::redis::vo::RedisParserVO;

use super::{ChainPollingTask, ScriptPollingTask};

/// 调度 Chain 和 Script 两个独立 Redis Hash 轮询任务。
///
/// 对应 Java:
/// `com.yomahub.liteflow.parser.redis.mode.polling.RedisParserPollingMode`。
#[derive(Clone)]
pub struct RedisParserPollingMode {
    parser: RedisXmlELParser,
    chain_task: ChainPollingTask,
    script_task: Option<ScriptPollingTask>,
    polling_start_time: Duration,
    polling_interval: Duration,
}

impl RedisParserPollingMode {
    /// 按 Java 配置创建并初始化 Hash 指纹缓存。
    ///
    /// 对应 Java `RedisParserPollingMode#RedisParserPollingMode`。
    pub fn new(config: RedisParserVO) -> LFResult<Self> {
        if config.polling_interval == 0 {
            return Err(LiteflowError::Rule(
                "redis pollingInterval must be greater than 0".to_string(),
            ));
        }
        let parser = RedisXmlELParser::new(config.clone())?;
        let chain_key = config
            .chain_key
            .clone()
            .ok_or_else(|| LiteflowError::Rule("redis chainKey is blank".to_string()))?;
        let chain_client =
            <Self as RedisParserHelper>::get_redis_client(&config, config.chain_data_base)?
                .ok_or_else(|| LiteflowError::Rule("redis chainDataBase is blank".to_string()))?;
        let chain_task = ChainPollingTask::new(chain_client, chain_key)?;
        let script_task = match config.script_key.clone() {
            Some(script_key) => {
                let script_client = <Self as RedisParserHelper>::get_redis_client(
                    &config,
                    config.script_data_base,
                )?
                .ok_or_else(|| LiteflowError::Rule("redis scriptDataBase is blank".to_string()))?;
                Some(ScriptPollingTask::new(script_client, script_key)?)
            }
            None => None,
        };
        Ok(Self {
            parser,
            chain_task,
            script_task,
            polling_start_time: Duration::from_secs(config.polling_start_time),
            polling_interval: Duration::from_secs(config.polling_interval),
        })
    }

    /// 启动独立 Tokio 轮询任务；调用返回句柄的 `abort` 并等待即可停止。
    ///
    /// 对应 Java `RedisParserPollingMode#listenRedis`。
    #[must_use]
    pub fn listen_redis(&self, watcher: RuleSourceWatcher) -> tokio::task::JoinHandle<()> {
        let chain_task = self.chain_task.clone();
        let script_task = self.script_task.clone();
        let polling_start_time = self.polling_start_time;
        let polling_interval = self.polling_interval;
        tokio::spawn(async move {
            tokio::time::sleep(polling_start_time).await;
            let mut interval = tokio::time::interval(polling_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if let Err(error) = chain_task.run(&watcher).await {
                    eprintln!("[liteflow] redis chain polling failed: {error}");
                }
                if let Some(script_task) = &script_task {
                    if let Err(error) = script_task.run(&watcher).await {
                        eprintln!("[liteflow] redis script polling failed: {error}");
                    }
                }
            }
        })
    }
}

impl RedisParserHelper for RedisParserPollingMode {
    /// 返回 Chain/Script Hash 聚合后的 XML。
    fn get_content(&self) -> LFResult<String> {
        self.parser.get_content()
    }
}

#[async_trait]
impl RuleSource for RedisParserPollingMode {
    /// 拉取聚合 XML 和指纹，供初始装载及轮询变化后的统一对账使用。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let mode = self.clone();
        let text = tokio::task::spawn_blocking(move || mode.get_content())
            .await
            .map_err(|error| LiteflowError::Rule(format!("redis task error: {error}")))??;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        RuleFormat::Xml
    }

    fn name(&self) -> &str {
        "redis"
    }
}
