//! Apollo 解析器扩展配置。

use serde::{Deserialize, Serialize};

use crate::parser::apollo::exception::ApolloException;

/// 保存 Chain namespace 与可选 Script namespace。
///
/// Jackson 驼峰字段通过 serde `rename_all` 保持兼容。
/// 对应 Java: `com.yomahub.liteflow.parser.apollo.vo.ApolloParserConfigVO`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApolloParserConfigVO {
    chain_namespace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    script_namespace: Option<String>,
}

impl ApolloParserConfigVO {
    /// 使用 Chain namespace 与可选 Script namespace 创建配置。
    ///
    /// 对应 Java `ApolloParserConfigVO#ApolloParserConfigVO(String,String)`。
    #[must_use]
    pub fn new(
        chain_namespace: impl Into<String>,
        script_namespace: Option<impl Into<String>>,
    ) -> Self {
        Self {
            chain_namespace: chain_namespace.into(),
            script_namespace: script_namespace.map(Into::into),
        }
    }

    /// 返回 Chain namespace。对应 Java `getChainNamespace`。
    #[must_use]
    pub fn chain_namespace(&self) -> &str {
        &self.chain_namespace
    }

    /// 设置 Chain namespace。参数语义对应 Java `chainNamespace`。
    pub fn set_chain_namespace(&mut self, chain_namespace: impl Into<String>) {
        self.chain_namespace = chain_namespace.into();
    }

    /// 返回可选 Script namespace。对应 Java `getScriptNamespace`。
    #[must_use]
    pub fn script_namespace(&self) -> Option<&str> {
        self.script_namespace.as_deref()
    }

    /// 设置可选 Script namespace。参数语义对应 Java `scriptNamespace`。
    pub fn set_script_namespace(&mut self, script_namespace: Option<impl Into<String>>) {
        self.script_namespace = script_namespace.map(Into::into);
    }

    /// 校验 Java 构造函数要求的必要配置。
    pub fn validate(&self) -> Result<(), ApolloException> {
        if self.chain_namespace.trim().is_empty() {
            return Err(ApolloException::new(
                "chainNamespace is empty, you must configure the chainNamespace property",
            ));
        }
        Ok(())
    }
}
