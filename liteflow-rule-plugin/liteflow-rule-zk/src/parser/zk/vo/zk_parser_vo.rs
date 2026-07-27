//! ZooKeeper 解析器扩展配置。

use serde::{Deserialize, Serialize};

use crate::parser::zk::exception::ZkException;

/// 保存 ZooKeeper 连接串与 Chain/Script 根路径。
///
/// serde 驼峰字段对齐 Jackson 配置。
/// 对应 Java: `com.yomahub.liteflow.parser.zk.vo.ZkParserVO`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZkParserVO {
    connect_str: String,
    chain_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    script_path: Option<String>,
}

impl ZkParserVO {
    /// 使用必要连接串和 Chain 根路径创建配置。
    #[must_use]
    pub fn new(connect_str: impl Into<String>, chain_path: impl Into<String>) -> Self {
        Self {
            connect_str: connect_str.into(),
            chain_path: chain_path.into(),
            script_path: None,
        }
    }

    /// 返回 ZooKeeper 连接串。对应 Java `getConnectStr`。
    #[must_use]
    pub fn connect_str(&self) -> &str {
        &self.connect_str
    }

    /// 设置 ZooKeeper 连接串。对应 Java `setConnectStr`。
    pub fn set_connect_str(&mut self, connect_str: impl Into<String>) {
        self.connect_str = connect_str.into();
    }

    /// 返回 Chain 根路径。对应 Java `getChainPath`。
    #[must_use]
    pub fn chain_path(&self) -> &str {
        &self.chain_path
    }

    /// 设置 Chain 根路径。对应 Java `setChainPath`。
    pub fn set_chain_path(&mut self, chain_path: impl Into<String>) {
        self.chain_path = chain_path.into();
    }

    /// 返回可选 Script 根路径。对应 Java `getScriptPath`。
    #[must_use]
    pub fn script_path(&self) -> Option<&str> {
        self.script_path.as_deref()
    }

    /// 设置可选 Script 根路径。对应 Java `setScriptPath`。
    pub fn set_script_path(&mut self, script_path: Option<impl Into<String>>) {
        self.script_path = script_path.map(Into::into);
    }

    /// 校验 Java 解析器要求的必要配置。
    pub fn validate(&self) -> Result<(), ZkException> {
        if self.chain_path.trim().is_empty() {
            return Err(ZkException::new(
                "You must configure the chainPath property",
            ));
        }
        if self.connect_str.trim().is_empty() {
            return Err(ZkException::new("zk connect string is empty"));
        }
        Ok(())
    }
}
