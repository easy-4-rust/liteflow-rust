//! Etcd 前缀树读取与监听辅助对象。

use liteflow_core::parser::helper::NodeConvertHelper;
use liteflow_core::rule_plugin::RuleSourceWatcher;
use liteflow_core::util::RuleParsePluginUtil;

use crate::parser::etcd::EtcdClient;
use crate::parser::etcd::exception::EtcdException;
use crate::parser::etcd::vo::EtcdParserVO;

/// 聚合 Etcd Chain/Script 子节点并安装 Watch。
///
/// 对应 Java: `com.yomahub.liteflow.parser.etcd.util.EtcdParserHelper`。
#[derive(Debug, Clone)]
pub struct EtcdParserHelper {
    config: EtcdParserVO,
    client: EtcdClient,
}

impl EtcdParserHelper {
    /// 使用扩展配置创建辅助对象。对应 Java `EtcdParserHelper#EtcdParserHelper`。
    pub fn new(config: EtcdParserVO) -> Result<Self, EtcdException> {
        config.validate()?;
        let client = EtcdClient::new(
            config.endpoint_list(),
            config.namespace().map(str::to_string),
            config.user().map(str::to_string),
            config.password().map(str::to_string),
        );
        Ok(Self { config, client })
    }

    /// 聚合 Chain/Script 前缀树为完整 XML。
    ///
    /// 对应 Java `EtcdParserHelper#getContent`。
    pub async fn get_content(&self) -> Result<String, EtcdException> {
        let mut chain_xml = String::new();
        for chain_name in self
            .client
            .get_children_keys(self.config.chain_path(), "/")
            .await?
        {
            if let Some(chain_data) = self
                .client
                .get(&format!("{}/{}", self.config.chain_path(), chain_name))
                .await?
                .filter(|value| !value.trim().is_empty())
            {
                chain_xml.push_str(
                    &RuleParsePluginUtil::parse_chain_key(&chain_name).to_el_xml(&chain_data),
                );
            }
        }

        let script_xml = if self.has_script().await {
            let script_path = self.config.script_path().unwrap_or_default();
            let mut nodes = String::new();
            for script_key in self.client.get_children_keys(script_path, "/").await? {
                if script_key.trim().is_empty() {
                    continue;
                }
                let mut node = NodeConvertHelper::convert(&script_key).ok_or_else(|| {
                    EtcdException::new(format!("The name of the etcd node is invalid:{script_key}"))
                })?;
                let script = self
                    .client
                    .get(&format!("{script_path}/{script_key}"))
                    .await?
                    .unwrap_or_default();
                node.set_script(script);
                nodes.push_str(&RuleParsePluginUtil::to_script_xml(&node));
            }
            format!("<nodes>{nodes}</nodes>")
        } else {
            String::new()
        };

        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><flow>{script_xml}{chain_xml}</flow>"
        ))
    }

    /// 判断是否配置且实际存在 Script 子节点。
    ///
    /// 查询失败时与 Java 一样返回 `false`。对应 Java `EtcdParserHelper#hasScript`。
    pub async fn has_script(&self) -> bool {
        let Some(script_path) = self
            .config
            .script_path()
            .filter(|path| !path.trim().is_empty())
        else {
            return false;
        };
        self.client
            .get_children_keys(script_path, "/")
            .await
            .is_ok_and(|items| !items.is_empty())
    }

    /// 监听 Chain 与可选 Script 前缀变化，并触发规则重载。
    ///
    /// 对应 Java `EtcdParserHelper#listen`。
    pub async fn listen(&self, watcher: RuleSourceWatcher) -> Result<(), EtcdException> {
        let update_watcher = watcher.clone();
        let delete_watcher = watcher.clone();
        self.client
            .watch_child_change(
                self.config.chain_path(),
                move |_, _| {
                    let watcher = update_watcher.clone();
                    tokio::spawn(async move {
                        let _ = watcher.reload().await;
                    });
                },
                move |_| {
                    let watcher = delete_watcher.clone();
                    tokio::spawn(async move {
                        let _ = watcher.reload().await;
                    });
                },
            )
            .await?;

        if let Some(script_path) = self
            .config
            .script_path()
            .filter(|path| !path.trim().is_empty())
        {
            let update_watcher = watcher.clone();
            let delete_watcher = watcher;
            self.client
                .watch_child_change(
                    script_path,
                    move |path, _| {
                        let script_key = path.rsplit('/').next().unwrap_or_default();
                        match NodeConvertHelper::convert(script_key) {
                            Some(node) if node.enable() => {
                                let watcher = update_watcher.clone();
                                tokio::spawn(async move {
                                    let _ = watcher.reload().await;
                                });
                            }
                            Some(node) => update_watcher.unload_script_node(node.node_id()),
                            None => {}
                        }
                    },
                    move |path| {
                        let script_key = path.rsplit('/').next().unwrap_or_default();
                        if let Some(node) = NodeConvertHelper::convert(script_key) {
                            delete_watcher.unload_script_node(node.node_id());
                        }
                    },
                )
                .await?;
        }
        Ok(())
    }

    /// 返回 Etcd 客户端。
    #[must_use]
    pub fn client(&self) -> &EtcdClient {
        &self.client
    }

    /// 返回解析配置。
    #[must_use]
    pub fn config(&self) -> &EtcdParserVO {
        &self.config
    }
}
