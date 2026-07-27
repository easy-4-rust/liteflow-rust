// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::quota_extractor::QuotaExtractor;
use crate::quota_metadata::QuotaMetadata;
use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;

/// 解析 Anthropic Claude API 限流响应的配额提取器。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `AnthropicQuotaExtractor`）。
pub struct AnthropicQuotaExtractor;

impl QuotaExtractor for AnthropicQuotaExtractor {
    fn extract_from_headers(&self, headers: &HeaderMap) -> Option<QuotaMetadata> {
        let rate_limit_remaining = headers
            .get("anthropic-ratelimit-requests-remaining")
            .or_else(|| headers.get("Anthropic-RateLimit-Requests-Remaining"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let rate_limit_total = headers
            .get("anthropic-ratelimit-requests-limit")
            .or_else(|| headers.get("Anthropic-RateLimit-Requests-Limit"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let rate_limit_reset_at = headers
            .get("anthropic-ratelimit-requests-reset")
            .or_else(|| headers.get("Anthropic-RateLimit-Requests-Reset"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|date_time| date_time.with_timezone(&Utc));
        let retry_after_seconds = headers
            .get("retry-after")
            .or_else(|| headers.get("Retry-After"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        (rate_limit_remaining.is_some()
            || rate_limit_reset_at.is_some()
            || retry_after_seconds.is_some())
        .then_some(QuotaMetadata {
            rate_limit_remaining,
            rate_limit_reset_at,
            retry_after_seconds,
            rate_limit_total,
        })
    }

    fn extract_from_error(&self, error: &anyhow::Error) -> Option<QuotaMetadata> {
        let error_message = error.to_string().to_lowercase();
        // Anthropic 的过载与限流错误未必携带头信息，使用保守的 60 秒回退。
        (error_message.contains("overloaded") || error_message.contains("rate limit")).then_some(
            QuotaMetadata {
                rate_limit_remaining: Some(0),
                rate_limit_reset_at: None,
                retry_after_seconds: Some(60),
                rate_limit_total: None,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AnthropicQuotaExtractor;
    use crate::quota_extractor::QuotaExtractor;
    use reqwest::header::HeaderMap;

    #[test]
    fn extracts_anthropic_headers() {
        let extractor = AnthropicQuotaExtractor;
        let mut headers = HeaderMap::new();
        headers.insert(
            "anthropic-ratelimit-requests-remaining",
            "50".parse().expect("valid header"),
        );
        headers.insert("retry-after", "30".parse().expect("valid header"));

        let quota = extractor
            .extract_from_headers(&headers)
            .expect("quota headers");
        assert_eq!(quota.rate_limit_remaining, Some(50));
        assert_eq!(quota.retry_after_seconds, Some(30));
    }
}
