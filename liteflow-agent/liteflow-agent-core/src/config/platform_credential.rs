use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 大模型平台凭证对象。
///
/// 头等平台直接存放在 `AgentConfig` 字段中，兼容平台则按用户自定义 key 存入 Map。
/// 具体 Provider 在构造 AgentScope 模型前负责校验 API Key。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.PlatformCredential`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlatformCredential {
    /// 平台 API Key；具体 Provider 使用时必须非空。
    pub api_key: Option<String>,
    /// 可选基础地址，用于网关、代理或私有化部署。
    pub base_url: Option<String>,
    /// 预留的扩展参数。
    pub extra: HashMap<String, String>,
}

impl PlatformCredential {
    /// 返回 API Key。对应 Java: `PlatformCredential#getApiKey`。
    #[must_use]
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// 设置 API Key。对应 Java: `PlatformCredential#setApiKey`。
    pub fn set_api_key(&mut self, api_key: Option<String>) {
        self.api_key = api_key;
    }

    /// 返回基础地址。对应 Java: `PlatformCredential#getBaseUrl`。
    #[must_use]
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// 设置基础地址。对应 Java: `PlatformCredential#setBaseUrl`。
    pub fn set_base_url(&mut self, base_url: Option<String>) {
        self.base_url = base_url;
    }

    /// 返回扩展参数。对应 Java: `PlatformCredential#getExtra`。
    #[must_use]
    pub fn extra(&self) -> &HashMap<String, String> {
        &self.extra
    }

    /// 设置扩展参数。对应 Java: `PlatformCredential#setExtra`。
    pub fn set_extra(&mut self, extra: HashMap<String, String>) {
        self.extra = extra;
    }
}
