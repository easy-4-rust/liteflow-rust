// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::quota_extractor::QuotaExtractor;
use crate::quota_metadata::QuotaMetadata;
use reqwest::header::HeaderMap;

/// 解析 Google Gemini API 限流响应的配额提取器。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `GeminiQuotaExtractor`）。
pub struct GeminiQuotaExtractor;

impl QuotaExtractor for GeminiQuotaExtractor {
    fn extract_from_headers(&self, headers: &HeaderMap) -> Option<QuotaMetadata> {
        let rate_limit_remaining = headers
            .get("X-Goog-RateLimit-Requests-Remaining")
            .or_else(|| headers.get("x-goog-ratelimit-requests-remaining"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let rate_limit_total = headers
            .get("X-Goog-RateLimit-Requests-Limit")
            .or_else(|| headers.get("x-goog-ratelimit-requests-limit"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let retry_after_seconds = headers
            .get("Retry-After")
            .or_else(|| headers.get("retry-after"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        (rate_limit_remaining.is_some() || retry_after_seconds.is_some()).then_some(QuotaMetadata {
            rate_limit_remaining,
            rate_limit_reset_at: None,
            retry_after_seconds,
            rate_limit_total,
        })
    }

    fn extract_from_error(&self, error: &anyhow::Error) -> Option<QuotaMetadata> {
        let error_message = error.to_string();
        // Gemini 配额耗尽错误通常没有重置时间，采用一小时回退等待。
        (error_message.contains("RESOURCE_EXHAUSTED")
            || error_message.contains("insufficient quota"))
        .then_some(QuotaMetadata {
            rate_limit_remaining: Some(0),
            rate_limit_reset_at: None,
            retry_after_seconds: Some(3600),
            rate_limit_total: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::GeminiQuotaExtractor;
    use crate::quota_extractor::QuotaExtractor;
    use reqwest::header::HeaderMap;

    #[test]
    fn extracts_gemini_headers_and_exhausted_error() {
        let extractor = GeminiQuotaExtractor;
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Goog-RateLimit-Requests-Remaining",
            "20".parse().expect("valid header"),
        );
        headers.insert(
            "X-Goog-RateLimit-Requests-Limit",
            "100".parse().expect("valid header"),
        );

        let quota = extractor
            .extract_from_headers(&headers)
            .expect("quota headers");
        assert_eq!(quota.rate_limit_remaining, Some(20));
        assert_eq!(quota.rate_limit_total, Some(100));

        let error = anyhow::anyhow!("gemini API error (429): RESOURCE_EXHAUSTED");
        assert_eq!(
            extractor
                .extract_from_error(&error)
                .expect("quota error")
                .retry_after_seconds,
            Some(3600)
        );
    }
}
