use std::collections::HashMap;

use crate::{AgentConfigException, PlatformCredential};

/// 为 Provider 模型描述符统一解析并校验平台凭证。
///
/// 缺失凭证时返回包含完整配置路径的 `AgentConfigException`，使 OpenAI、
/// Anthropic、Gemini、DashScope 及兼容平台保持一致的诊断信息。
///
/// 对应 Java: `com.yomahub.liteflow.agent.model.CredentialResolver`。
pub struct CredentialResolver;

impl CredentialResolver {
    /// 获取头等平台凭证并校验 API Key。
    ///
    /// # 参数
    /// - `credential`: 从 `AgentConfig` 取得的凭证；允许 `None` 以对齐 Java 空值输入。
    /// - `config_path`: 配置路径前缀，例如 `liteflow.agent.openai`。
    ///
    /// # 返回
    /// 校验通过的原凭证引用；缺失或 API Key 为空时返回带配置路径的配置错误。
    ///
    /// 对应 Java: `CredentialResolver#requireFirstClass`。
    pub fn require_first_class<'a>(
        credential: Option<&'a PlatformCredential>,
        config_path: &str,
    ) -> Result<&'a PlatformCredential, AgentConfigException> {
        let credential = credential.filter(|value| {
            value
                .api_key()
                .is_some_and(|api_key| !api_key.trim().is_empty())
        });
        credential.ok_or_else(|| {
            AgentConfigException::new(format!(
                "Missing API key: please configure {config_path}.api-key"
            ))
        })
    }

    /// 获取兼容平台凭证并校验 API Key。
    ///
    /// # 参数
    /// - `credentials`: 兼容平台 key 到凭证的映射；允许 `None` 以对齐 Java 空值输入。
    /// - `key`: 平台 key，例如 `deepseek`。
    /// - `config_path`: 配置路径前缀，例如 `liteflow.agent.openai-compatible`。
    ///
    /// # 返回
    /// 校验通过的原凭证引用。缺少平台项与平台存在但 API Key 为空使用不同错误消息。
    ///
    /// 对应 Java: `CredentialResolver#requireCompatible`。
    pub fn require_compatible<'a>(
        credentials: Option<&'a HashMap<String, PlatformCredential>>,
        key: &str,
        config_path: &str,
    ) -> Result<&'a PlatformCredential, AgentConfigException> {
        let credential = credentials
            .and_then(|values| values.get(key))
            .ok_or_else(|| {
                AgentConfigException::new(format!(
                    "Missing platform credential: please configure {config_path}.{key}.api-key"
                ))
            })?;
        if credential
            .api_key()
            .is_none_or(|api_key| api_key.trim().is_empty())
        {
            return Err(AgentConfigException::new(format!(
                "Missing API key: please configure {config_path}.{key}.api-key"
            )));
        }
        Ok(credential)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{CredentialResolver, PlatformCredential};

    #[test]
    fn first_class_requires_non_blank_api_key_with_exact_path() {
        let error = CredentialResolver::require_first_class(None, "liteflow.agent.openai")
            .expect_err("缺失头等平台凭证应失败");
        assert_eq!(
            error.to_string(),
            "Missing API key: please configure liteflow.agent.openai.api-key"
        );

        let mut credential = PlatformCredential::default();
        credential.set_api_key(Some("  secret  ".to_string()));
        assert_eq!(
            CredentialResolver::require_first_class(Some(&credential), "liteflow.agent.openai")
                .expect("非空 API Key 应通过")
                .api_key(),
            Some("  secret  ")
        );
    }

    #[test]
    fn compatible_distinguishes_missing_platform_from_blank_api_key() {
        let credentials = HashMap::new();
        let missing = CredentialResolver::require_compatible(
            Some(&credentials),
            "deepseek",
            "liteflow.agent.openai-compatible",
        )
        .expect_err("缺少兼容平台项应失败");
        assert_eq!(
            missing.to_string(),
            "Missing platform credential: please configure \
             liteflow.agent.openai-compatible.deepseek.api-key"
        );

        let credentials = HashMap::from([("deepseek".to_string(), PlatformCredential::default())]);
        let blank = CredentialResolver::require_compatible(
            Some(&credentials),
            "deepseek",
            "liteflow.agent.openai-compatible",
        )
        .expect_err("空 API Key 应失败");
        assert_eq!(
            blank.to_string(),
            "Missing API key: please configure liteflow.agent.openai-compatible.deepseek.api-key"
        );
    }
}
