// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::provider_quota_info::ProviderQuotaInfo;
use crate::quota_status::QuotaStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 指定时刻所有提供商的配额状态汇总。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `QuotaSummary`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSummary {
    /// 汇总生成时刻。
    pub timestamp: DateTime<Utc>,
    /// 各提供商的配额状态。
    pub providers: Vec<ProviderQuotaInfo>,
}

impl QuotaSummary {
    /// 返回当前健康可用的提供商名称。
    ///
    /// # 返回
    ///
    /// 状态为 [`QuotaStatus::Ok`] 的提供商名称列表。
    pub fn available_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|provider| provider.status == QuotaStatus::Ok)
            .map(|provider| provider.provider.as_str())
            .collect()
    }

    /// 返回当前被限流或配额耗尽的提供商名称。
    ///
    /// # 返回
    ///
    /// 状态为 `RateLimited` 或 `QuotaExhausted` 的提供商名称列表。
    pub fn rate_limited_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|provider| {
                provider.status == QuotaStatus::RateLimited
                    || provider.status == QuotaStatus::QuotaExhausted
            })
            .map(|provider| provider.provider.as_str())
            .collect()
    }

    /// 返回当前熔断器已经打开的提供商名称。
    ///
    /// # 返回
    ///
    /// 状态为 [`QuotaStatus::CircuitOpen`] 的提供商名称列表。
    pub fn circuit_open_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|provider| provider.status == QuotaStatus::CircuitOpen)
            .map(|provider| provider.provider.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::QuotaSummary;
    use crate::provider_quota_info::ProviderQuotaInfo;
    use crate::quota_status::QuotaStatus;
    use chrono::Utc;

    fn provider(name: &str, status: QuotaStatus) -> ProviderQuotaInfo {
        ProviderQuotaInfo {
            provider: name.to_string(),
            status,
            failure_count: 0,
            last_error: None,
            retry_after_seconds: None,
            circuit_resets_at: None,
            profiles: Vec::new(),
        }
    }

    #[test]
    fn groups_providers_by_quota_status() {
        let summary = QuotaSummary {
            timestamp: Utc::now(),
            providers: vec![
                provider("openai", QuotaStatus::Ok),
                provider("anthropic", QuotaStatus::RateLimited),
                provider("qwen", QuotaStatus::QuotaExhausted),
                provider("gemini", QuotaStatus::CircuitOpen),
            ],
        };

        assert_eq!(summary.available_providers(), vec!["openai"]);
        assert_eq!(summary.rate_limited_providers(), vec!["anthropic", "qwen"]);
        assert_eq!(summary.circuit_open_providers(), vec!["gemini"]);
    }
}
