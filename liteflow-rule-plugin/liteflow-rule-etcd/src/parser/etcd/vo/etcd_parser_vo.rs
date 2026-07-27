//! Etcd 规则源扩展配置。

use serde::{Deserialize, Serialize};

use crate::parser::etcd::exception::EtcdException;

/// 保存 endpoints、认证、namespace 与 Chain/Script 路径。
///
/// serde 驼峰字段对齐 Jackson 配置。
/// 对应 Java: `com.yomahub.liteflow.parser.etcd.vo.EtcdParserVO`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EtcdParserVO {
    endpoints: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    chain_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    script_path: Option<String>,
}

impl EtcdParserVO {
    /// 使用必要 endpoints 与 Chain 路径创建配置。
    #[must_use]
    pub fn new(endpoints: impl Into<String>, chain_path: impl Into<String>) -> Self {
        Self {
            endpoints: endpoints.into(),
            chain_path: chain_path.into(),
            ..Self::default()
        }
    }

    /// 返回逗号分隔 endpoints。对应 Java `getEndpoints`。
    #[must_use]
    pub fn endpoints(&self) -> &str {
        &self.endpoints
    }

    /// 返回拆分、去空白后的 endpoint 列表。
    #[must_use]
    pub fn endpoint_list(&self) -> Vec<String> {
        self.endpoints
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// 设置逗号分隔 endpoints。对应 Java `setEndpoints`。
    pub fn set_endpoints(&mut self, endpoints: impl Into<String>) {
        self.endpoints = endpoints.into();
    }

    /// 返回可选用户。对应 Java `getUser`。
    #[must_use]
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// 设置可选用户。对应 Java `setUser`。
    pub fn set_user(&mut self, user: Option<impl Into<String>>) {
        self.user = user.map(Into::into);
    }

    /// 返回可选密码。对应 Java `getPassword`。
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// 设置可选密码。对应 Java `setPassword`。
    pub fn set_password(&mut self, password: Option<impl Into<String>>) {
        self.password = password.map(Into::into);
    }

    /// 返回可选 namespace。对应 Java `getNamespace`。
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// 设置可选 namespace。对应 Java `setNamespace`。
    pub fn set_namespace(&mut self, namespace: Option<impl Into<String>>) {
        self.namespace = namespace.map(Into::into);
    }

    /// 返回 Chain 路径。对应 Java `getChainPath`。
    #[must_use]
    pub fn chain_path(&self) -> &str {
        &self.chain_path
    }

    /// 设置 Chain 路径。对应 Java `setChainPath`。
    pub fn set_chain_path(&mut self, chain_path: impl Into<String>) {
        self.chain_path = chain_path.into();
    }

    /// 返回可选 Script 路径。对应 Java `getScriptPath`。
    #[must_use]
    pub fn script_path(&self) -> Option<&str> {
        self.script_path.as_deref()
    }

    /// 设置可选 Script 路径。对应 Java `setScriptPath`。
    pub fn set_script_path(&mut self, script_path: Option<impl Into<String>>) {
        self.script_path = script_path.map(Into::into);
    }

    /// 校验 Java 解析器要求的必要配置。
    pub fn validate(&self) -> Result<(), EtcdException> {
        if self.chain_path.trim().is_empty() {
            return Err(EtcdException::new(
                "You must configure the chainPath property",
            ));
        }
        if self.endpoint_list().is_empty() {
            return Err(EtcdException::new("etcd endpoints is empty"));
        }
        Ok(())
    }
}
