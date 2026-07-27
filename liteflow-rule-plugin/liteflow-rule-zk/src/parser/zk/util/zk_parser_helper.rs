//! ZooKeeper 子节点读取、XML 聚合与监听辅助对象。

use std::sync::Arc;
use std::time::Duration;

use liteflow_core::parser::helper::NodeConvertHelper;
use liteflow_core::rule_plugin::RuleSourceWatcher;
use liteflow_core::util::RuleParsePluginUtil;
use zookeeper::{AddWatchMode, WatchedEvent, WatchedEventType, ZooKeeper};

use crate::parser::zk::exception::ZkException;
use crate::parser::zk::nop_watcher::NopWatcher;
use crate::parser::zk::vo::ZkParserVO;

/// 建立并复用 ZooKeeper 会话，聚合 Chain/Script 子节点并安装递归 Watch。
///
/// 对应 Java: `com.yomahub.liteflow.parser.zk.util.ZkParserHelper`。
#[derive(Clone)]
pub struct ZkParserHelper {
    config: ZkParserVO,
    client: Arc<ZooKeeper>,
}

impl ZkParserHelper {
    /// 校验配置并建立 ZooKeeper 会话。
    ///
    /// 会话超时为 5 秒，对应 Java Curator 客户端的重试连接语义。
    /// 对应 Java `ZkParserHelper#ZkParserHelper`。
    pub fn new(config: ZkParserVO) -> Result<Self, ZkException> {
        config.validate()?;
        let client = ZooKeeper::connect(config.connect_str(), Duration::from_secs(5), NopWatcher)?;
        Ok(Self {
            config,
            client: Arc::new(client),
        })
    }

    /// 读取 Chain/Script 子节点并聚合为完整 XML。
    ///
    /// Chain 子节点名使用 `chainId:enable`，Script 子节点名使用
    /// `id:type:name:language:enable`。
    /// 对应 Java `ZkParserHelper#getContent`。
    pub fn get_content(&self) -> Result<String, ZkException> {
        if self
            .client
            .exists(self.config.chain_path(), false)?
            .is_none()
        {
            return Err(ZkException::new(format!(
                "zk node[{}] is not exist",
                self.config.chain_path()
            )));
        }

        let mut chain_names = self.client.get_children(self.config.chain_path(), false)?;
        chain_names.sort();
        let chain_xml = chain_names
            .into_iter()
            .map(|chain_name| {
                let path = child_path(self.config.chain_path(), &chain_name);
                let (data, _) = self.client.get_data(&path, false)?;
                let content = String::from_utf8(data)?;
                if content.trim().is_empty() {
                    Ok(String::new())
                } else {
                    Ok(RuleParsePluginUtil::parse_chain_key(&chain_name).to_el_xml(&content))
                }
            })
            .collect::<Result<String, ZkException>>()?;

        let script_xml = if self.has_script() {
            let script_path = self.config.script_path().expect("已确认 Script 路径存在");
            let mut script_names = self.client.get_children(script_path, false)?;
            script_names.sort();
            let items = script_names
                .into_iter()
                .map(|script_name| {
                    let mut node = NodeConvertHelper::convert(&script_name).ok_or_else(|| {
                        ZkException::new(format!(
                            "The name of the zk node is invalid:{script_name}"
                        ))
                    })?;
                    let path = child_path(script_path, &script_name);
                    let (data, _) = self.client.get_data(&path, false)?;
                    node.set_script(String::from_utf8(data)?);
                    Ok(RuleParsePluginUtil::to_script_xml(&node))
                })
                .collect::<Result<String, ZkException>>()?;
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
    pub fn has_script(&self) -> bool {
        let Some(script_path) = self
            .config
            .script_path()
            .filter(|path| !path.trim().is_empty())
        else {
            return false;
        };
        self.client
            .exists(script_path, false)
            .ok()
            .flatten()
            .is_some()
            && self
                .client
                .get_children(script_path, false)
                .is_ok_and(|children| !children.is_empty())
    }

    /// 安装 Chain 与可选 Script 根路径的持久递归 Watch。
    ///
    /// ZooKeeper 3.6+ 会持续推送后代节点增删改事件；Chain 事件统一重载并
    /// 对账，Script 禁用或删除会显式卸载节点。
    /// 对应 Java `ZkParserHelper#listenZkNode`。
    pub fn listen_zk_node(&self, watcher: RuleSourceWatcher) -> Result<(), ZkException> {
        let runtime = tokio::runtime::Handle::current();
        let chain_watcher = watcher.clone();
        let chain_runtime = runtime.clone();
        self.client.add_watch(
            self.config.chain_path(),
            AddWatchMode::PersistentRecursive,
            move |event: WatchedEvent| {
                if is_data_event(event.event_type) {
                    let watcher = chain_watcher.clone();
                    chain_runtime.spawn(async move {
                        if let Err(error) = watcher.reload().await {
                            eprintln!("[liteflow] zk chain reload failed: {error}");
                        }
                    });
                }
            },
        )?;

        if let Some(script_path) = self
            .config
            .script_path()
            .filter(|path| !path.trim().is_empty())
        {
            let script_watcher = watcher;
            self.client.add_watch(
                script_path,
                AddWatchMode::PersistentRecursive,
                move |event: WatchedEvent| {
                    if !is_data_event(event.event_type) {
                        return;
                    }
                    let Some(path) = event.path.as_deref() else {
                        return;
                    };
                    let script_name = path.rsplit('/').next().unwrap_or_default();
                    match NodeConvertHelper::convert(script_name) {
                        Some(node)
                            if event.event_type == WatchedEventType::NodeDeleted
                                || !node.enable() =>
                        {
                            script_watcher.unload_script_node(node.node_id());
                        }
                        Some(_) => {
                            let watcher = script_watcher.clone();
                            runtime.spawn(async move {
                                if let Err(error) = watcher.reload().await {
                                    eprintln!("[liteflow] zk script reload failed: {error}");
                                }
                            });
                        }
                        None => {}
                    }
                },
            )?;
        }
        Ok(())
    }

    /// 返回 ZooKeeper 客户端。
    #[must_use]
    pub fn client(&self) -> &Arc<ZooKeeper> {
        &self.client
    }

    /// 返回解析器配置。
    #[must_use]
    pub fn config(&self) -> &ZkParserVO {
        &self.config
    }
}

fn child_path(parent: &str, child: &str) -> String {
    format!("{}/{}", parent.trim_end_matches('/'), child)
}

fn is_data_event(event_type: WatchedEventType) -> bool {
    matches!(
        event_type,
        WatchedEventType::NodeCreated
            | WatchedEventType::NodeDataChanged
            | WatchedEventType::NodeDeleted
            | WatchedEventType::NodeChildrenChanged
    )
}
