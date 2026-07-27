// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::quota_extractor::QuotaExtractor;
use crate::quota_metadata::QuotaMetadata;
use reqwest::header::HeaderMap;

/// 解析 Qwen OAuth API 错误的配额提取器。
///
/// Qwen OAuth API 不返回限流响应头；免费层的已知额度为每日 1000 次请求。
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `QwenQuotaExtractor`）。
pub struct QwenQuotaExtractor;

impl QuotaExtractor for QwenQuotaExtractor {
    fn extract_from_headers(&self, _headers: &HeaderMap) -> Option<QuotaMetadata> {
        // 不制造静态响应头结果，避免截断通用提取器的回退链。
        None
    }

    fn extract_from_error(&self, error: &anyhow::Error) -> Option<QuotaMetadata> {
        let error_message = error.to_string().to_lowercase();
        (error_message.contains("too many requests")
            || error_message.contains("rate limit")
            || error_message.contains("quota"))
        .then_some(QuotaMetadata {
            rate_limit_remaining: Some(0),
            rate_limit_reset_at: None,
            retry_after_seconds: Some(3600),
            rate_limit_total: Some(1000),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::QwenQuotaExtractor;
    use crate::quota_extractor::QuotaExtractor;
    use reqwest::header::HeaderMap;

    #[test]
    fn ignores_headers_and_extracts_qwen_rate_limit_error() {
        let extractor = QwenQuotaExtractor;
        assert!(extractor.extract_from_headers(&HeaderMap::new()).is_none());

        let error = anyhow::anyhow!("qwen API error (429): Too many requests");
        let quota = extractor.extract_from_error(&error).expect("quota error");
        assert_eq!(quota.rate_limit_remaining, Some(0));
        assert_eq!(quota.rate_limit_total, Some(1000));
        assert_eq!(quota.retry_after_seconds, Some(3600));
    }
}
