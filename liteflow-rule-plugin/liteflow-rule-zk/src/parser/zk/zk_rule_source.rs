//! 对应 Java: `com.yomahub.liteflow.parser.zk.ZkParser`。

use async_trait::async_trait;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::rule_plugin::{RuleFormat, RuleSource, fnv_fp};

use super::nop_watcher::NopWatcher;

/// ZooKeeper znode 规则源。
pub struct ZkRuleSource {
    /// connect string，如 `127.0.0.1:2181`。
    pub connect_string: String,
    /// 规则节点路径，LiteFlow 默认使用 `/lite-flow/flow`。
    pub node_path: String,
    pub format: RuleFormat,
}

#[async_trait]
impl RuleSource for ZkRuleSource {
    /// 读取 znode 数据。对应 Java `ZkParserHelper#getContent`。
    async fn fetch(&self) -> LFResult<(String, String)> {
        let connect_string = self.connect_string.clone();
        let node_path = self.node_path.clone();
        let text = tokio::task::spawn_blocking(move || -> LFResult<String> {
            let client = zookeeper::ZooKeeper::connect(
                &connect_string,
                std::time::Duration::from_secs(5),
                NopWatcher,
            )
            .map_err(|error| LiteflowError::Rule(format!("zk connect error: {error}")))?;
            let (data, _) = client
                .get_data(&node_path, false)
                .map_err(|error| LiteflowError::Rule(format!("zk get data error: {error}")))?;
            client.close().ok();
            String::from_utf8(data)
                .map_err(|error| LiteflowError::Rule(format!("zk decode error: {error}")))
        })
        .await
        .map_err(|error| LiteflowError::Rule(format!("zk task error: {error}")))??;
        Ok((text.clone(), fnv_fp(&text)))
    }

    fn format(&self) -> RuleFormat {
        self.format
    }

    fn name(&self) -> &str {
        "zookeeper"
    }
}
