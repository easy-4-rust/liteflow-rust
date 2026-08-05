//! ZooKeeper 子节点读取、XML 聚合与监听辅助对象。

use std::sync::Arc;
use std::time::Duration;

use liteflow_core::parser::helper::NodeConvertHelper;
use liteflow_core::rule_plugin::RuleSourceWatcher;
use liteflow_core::util::RuleParsePluginUtil;
use tokio::sync::RwLock;
use zookeeper_client::{AddWatchMode, Client, EventType};

use crate::parser::zk::exception::ZkException;
use crate::parser::zk::vo::ZkParserVO;

/// 会话超时，对应 Java Curator 客户端的重试连接语义。
const SESSION_TIMEOUT: Duration = Duration::from_secs(5);

/// 建立并复用 ZooKeeper 会话，聚合 Chain/Script 子节点并安装递归 Watch。
///
/// 会话首次使用时按需建立；Watch 由独立任务驱动，不阻塞调用方。
/// 对应 Java: `com.yomahub.liteflow.parser.zk.util.ZkParserHelper`。
#[derive(Clone)]
pub struct ZkParserHelper {
    config: ZkParserVO,
    client: Arc<RwLock<Option<Arc<Client>>>>,
}

impl ZkParserHelper {
    /// 校验配置并创建辅助对象，会话在首次使用时建立。
    ///
    /// 对应 Java `ZkParserHelper#ZkParserHelper`。
    pub fn new(config: ZkParserVO) -> Result<Self, ZkException> {
        config.validate()?;
        Ok(Self {
            config,
            client: Arc::new(RwLock::new(None)),
        })
    }

    /// 建立并返回 ZooKeeper 会话，后续调用复用。
    ///
    /// 对应 Java Curator 客户端的懒连接语义。
    pub async fn client(&self) -> Result<Arc<Client>, ZkException> {
        if let Some(client) = self.client.read().await.as_ref() {
            return Ok(Arc::clone(client));
        }
        let client = Arc::new(
            Client::connector()
                .with_session_timeout(SESSION_TIMEOUT)
                .connect(self.config.connect_str())
                .await
                .map_err(ZkException::from)?,
        );
        *self.client.write().await = Some(Arc::clone(&client));
        Ok(client)
    }

    /// 读取 Chain/Script 子节点并聚合为完整 XML。
    ///
    /// Chain 子节点名使用 `chainId:enable`，Script 子节点名使用
    /// `id:type:name:language:enable`。
    /// 对应 Java `ZkParserHelper#getContent`。
    pub async fn get_content(&self) -> Result<String, ZkException> {
        let client = self.client().await?;
        let chain_path = self.config.chain_path().to_owned();
        let mut chain_names = match client.get_children(&chain_path).await {
            Ok((children, _)) => children,
            Err(zookeeper_client::Error::NoNode) => {
                return Err(ZkException::new(format!(
                    "zk node[{chain_path}] is not exist"
                )));
            }
            Err(error) => return Err(error.into()),
        };
        chain_names.sort();
        let mut chain_xml = String::new();
        for chain_name in chain_names {
            let path = child_path(&chain_path, &chain_name);
            let content = get_node_content(&client, &path).await?;
            if content.trim().is_empty() {
                continue;
            }
            chain_xml
                .push_str(&RuleParsePluginUtil::parse_chain_key(&chain_name).to_el_xml(&content));
        }

        let script_xml = if self.has_script().await {
            let script_path = self.config.script_path().expect("已确认 Script 路径存在");
            let mut script_names = client.get_children(script_path).await?.0;
            script_names.sort();
            let mut items = String::new();
            for script_name in script_names {
                let mut node = NodeConvertHelper::convert(&script_name).ok_or_else(|| {
                    ZkException::new(format!("The name of the zk node is invalid:{script_name}"))
                })?;
                let path = child_path(script_path, &script_name);
                let content = get_node_content(&client, &path).await?;
                node.set_script(content);
                items.push_str(&RuleParsePluginUtil::to_script_xml(&node));
            }
            format!("<nodes>{items}</nodes>")
        } else {
            String::new()
        };

        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><flow>{script_xml}{chain_xml}</flow>"
        ))
    }

    /// 判断 Script 路径已配置、存在且包含子节点。
    ///
    /// 查询异常与 Java 一样按不存在处理。对应 Java `ZkParserHelper#hasScript`。
    #[must_use]
    pub async fn has_script(&self) -> bool {
        let Some(script_path) = self
            .config
            .script_path()
            .filter(|path| !path.trim().is_empty())
        else {
            return false;
        };
        let Ok(client) = self.client().await else {
            return false;
        };
        client
            .get_children(script_path)
            .await
            .is_ok_and(|(children, _)| !children.is_empty())
    }

    /// 安装 Chain 与可选 Script 根路径的持久递归 Watch。
    ///
    /// ZooKeeper 3.6+ 会持续推送后代节点增删改事件；Chain 事件统一重载并
    /// 对账，Script 禁用或删除会显式卸载节点。Watch 在后台任务中消费事件，
    /// 会话终止后任务自行退出。
    /// 对应 Java `ZkParserHelper#listenZkNode`。
    pub async fn listen_zk_node(&self, watcher: RuleSourceWatcher) -> Result<(), ZkException> {
        let client = self.client().await?;
        let chain_watcher = watcher.clone();
        let chain_client = Arc::clone(&client);
        let chain_path = self.config.chain_path().to_owned();
        tokio::spawn(async move {
            Self::watch_chain(chain_client, chain_path, chain_watcher).await;
        });

        if let Some(script_path) = self
            .config
            .script_path()
            .filter(|path| !path.trim().is_empty())
        {
            let script_watcher = watcher;
            let script_client = Arc::clone(&client);
            let script_path = script_path.to_owned();
            tokio::spawn(async move {
                Self::watch_script(script_client, script_path, script_watcher).await;
            });
        }
        Ok(())
    }

    /// 消费 Chain 根路径的持久递归 Watch，数据事件统一触发重载。
    async fn watch_chain(client: Arc<Client>, path: String, watcher: RuleSourceWatcher) {
        let mut watch = match client.watch(&path, AddWatchMode::PersistentRecursive).await {
            Ok(watch) => watch,
            Err(error) => {
                eprintln!("[liteflow] zk chain watch install failed on {path}: {error}");
                return;
            }
        };
        loop {
            let event = watch.changed().await;
            if event.event_type == EventType::Session {
                if event.session_state.is_terminated() {
                    return;
                }
                continue;
            }
            if !is_data_event(event.event_type) {
                continue;
            }
            if let Err(error) = watcher.reload().await {
                eprintln!("[liteflow] zk chain reload failed: {error}");
            }
        }
    }

    /// 消费 Script 根路径的持久递归 Watch，禁用或删除的脚本显式卸载。
    async fn watch_script(client: Arc<Client>, path: String, watcher: RuleSourceWatcher) {
        let mut watch = match client.watch(&path, AddWatchMode::PersistentRecursive).await {
            Ok(watch) => watch,
            Err(error) => {
                eprintln!("[liteflow] zk script watch install failed on {path}: {error}");
                return;
            }
        };
        loop {
            let event = watch.changed().await;
            if event.event_type == EventType::Session {
                if event.session_state.is_terminated() {
                    return;
                }
                continue;
            }
            if !is_data_event(event.event_type) {
                continue;
            }
            let script_name = event.path.rsplit('/').next().unwrap_or_default();
            match NodeConvertHelper::convert(script_name) {
                Some(node) if event.event_type == EventType::NodeDeleted || !node.enable() => {
                    watcher.unload_script_node(node.node_id());
                }
                Some(_) => {
                    if let Err(error) = watcher.reload().await {
                        eprintln!("[liteflow] zk script reload failed: {error}");
                    }
                }
                None => {}
            }
        }
    }

    /// 返回解析器配置。
    #[must_use]
    pub fn config(&self) -> &ZkParserVO {
        &self.config
    }
}

/// 读取节点内容并转换为字符串。
async fn get_node_content(client: &Client, path: &str) -> Result<String, ZkException> {
    let (data, _) = client.get_data(path).await?;
    Ok(String::from_utf8(data)?)
}

fn child_path(parent: &str, child: &str) -> String {
    format!("{}/{}", parent.trim_end_matches('/'), child)
}

fn is_data_event(event_type: EventType) -> bool {
    matches!(
        event_type,
        EventType::NodeCreated
            | EventType::NodeDataChanged
            | EventType::NodeDeleted
            | EventType::NodeChildrenChanged
    )
}
