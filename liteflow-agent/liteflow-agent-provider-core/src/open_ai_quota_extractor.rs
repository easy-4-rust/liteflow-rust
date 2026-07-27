// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::quota_extractor::QuotaExtractor;
use crate::quota_metadata::QuotaMetadata;
use chrono::DateTime;
use reqwest::header::HeaderMap;

/// 解析 OpenAI 兼容接口限流响应的配额提取器。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `OpenAIQuotaExtractor`）。
pub struct OpenAIQuotaExtractor;

impl QuotaExtractor for OpenAIQuotaExtractor {
    fn extract_from_headers(&self, headers: &HeaderMap) -> Option<QuotaMetadata> {
        let rate_limit_remaining = headers
            .get("X-RateLimit-Remaining")
            .or_else(|| headers.get("x-ratelimit-remaining"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let rate_limit_total = headers
            .get("X-RateLimit-Limit")
            .or_else(|| headers.get("x-ratelimit-limit"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let rate_limit_reset_at = headers
            .get("X-RateLimit-Reset")
            .or_else(|| headers.get("x-ratelimit-reset"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok())
            .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0));
        let retry_after_seconds = headers
            .get("Retry-After")
            .or_else(|| headers.get("retry-after"))
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
        let error_message = error.to_string();
        // OpenAI 错误可能直接携带 “retry after N seconds”，提取其中第一个整数。
        let retry_after_seconds = (error_message.contains("retry after")
            || error_message.contains("Retry after"))
        .then(|| {
            error_message
                .split_whitespace()
                .find_map(|word| word.parse::<u64>().ok())
        })
        .flatten();

        retry_after_seconds.map(|retry_after_seconds| QuotaMetadata {
            rate_limit_remaining: Some(0),
            rate_limit_reset_at: None,
            retry_after_seconds: Some(retry_after_seconds),
            rate_limit_total: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::OpenAIQuotaExtractor;
    use crate::quota_extractor::QuotaExtractor;
    use reqwest::header::HeaderMap;

    #[test]
    fn extracts_open_ai_headers_and_retry_error() {
        let extractor = OpenAIQuotaExtractor;
        let mut headers = HeaderMap::new();
        headers.insert("X-RateLimit-Remaining", "10".parse().expect("valid header"));
        headers.insert("X-RateLimit-Limit", "100".parse().expect("valid header"));
        headers.insert(
            "X-RateLimit-Reset",
            "1708718400".parse().expect("valid header"),
        );

        let quota = extractor
            .extract_from_headers(&headers)
            .expect("quota headers");
        assert_eq!(quota.rate_limit_remaining, Some(10));
        assert_eq!(quota.rate_limit_total, Some(100));
        assert!(quota.rate_limit_reset_at.is_some());

        let error = anyhow::anyhow!("retry after 12 seconds");
        assert_eq!(
            extractor
                .extract_from_error(&error)
                .expect("quota error")
                .retry_after_seconds,
            Some(12)
        );
    }
}
