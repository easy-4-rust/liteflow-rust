// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// AWS SigV4 签名使用的访问密钥、可选会话令牌与区域。
///
/// 对应 Java: 无（Rust Bedrock 提供商基础设施；源自 ZeroClaw `AwsCredentials`）。
pub struct AwsCredentials {
    /// AWS Access Key ID。
    pub access_key_id: String,
    /// AWS Secret Access Key。
    pub secret_access_key: String,
    /// 可选 STS 会话令牌。
    pub session_token: Option<String>,
    /// AWS 区域。
    pub region: String,
}

impl AwsCredentials {
    /// 从标准 AWS 环境变量解析凭证。
    pub fn from_env() -> anyhow::Result<Self> {
        let access_key_id = std::env::var("AWS_ACCESS_KEY_ID")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("AWS_ACCESS_KEY_ID is required for Bedrock"))?;
        let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY is required for Bedrock"))?;
        let session_token = std::env::var("AWS_SESSION_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "us-east-1".to_string());
        Ok(Self {
            access_key_id,
            secret_access_key,
            session_token,
            region,
        })
    }

    /// 从显式参数创建长期 AWS 凭证。
    pub fn new(
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            session_token: None,
            region: region.into(),
        }
    }

    /// 附加 STS 临时会话令牌。
    #[must_use]
    pub fn with_session_token(mut self, token: impl Into<String>) -> Self {
        self.session_token = Some(token.into());
        self
    }
}
