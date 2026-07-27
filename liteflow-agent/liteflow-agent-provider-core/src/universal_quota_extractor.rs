// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::anthropic_quota_extractor::AnthropicQuotaExtractor;
use crate::gemini_quota_extractor::GeminiQuotaExtractor;
use crate::open_ai_quota_extractor::OpenAIQuotaExtractor;
use crate::quota_extractor::QuotaExtractor;
use crate::quota_metadata::QuotaMetadata;
use crate::qwen_quota_extractor::QwenQuotaExtractor;
use reqwest::header::HeaderMap;
use std::collections::HashMap;

/// 组合提供商专用提取器与兼容格式回退链的通用配额提取器。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `UniversalQuotaExtractor`）。
pub struct UniversalQuotaExtractor {
    extractors: HashMap<String, Box<dyn QuotaExtractor>>,
}

impl UniversalQuotaExtractor {
    /// 创建包含内置提供商映射的通用提取器。
    ///
    /// # 返回
    ///
    /// 已注册 OpenAI、Anthropic、Gemini 与 Qwen 及其兼容别名的提取器。
    pub fn new() -> Self {
        let mut extractors: HashMap<String, Box<dyn QuotaExtractor>> = HashMap::new();

        // 同一协议的别名分别注册，使调用方仍可按实际 provider 名称查找。
        extractors.insert("openai".to_string(), Box::new(OpenAIQuotaExtractor));
        extractors.insert("openai-codex".to_string(), Box::new(OpenAIQuotaExtractor));
        extractors.insert("anthropic".to_string(), Box::new(AnthropicQuotaExtractor));
        extractors.insert("gemini".to_string(), Box::new(GeminiQuotaExtractor));
        extractors.insert("openrouter".to_string(), Box::new(OpenAIQuotaExtractor));
        extractors.insert("qwen".to_string(), Box::new(QwenQuotaExtractor));
        extractors.insert("qwen-coding-plan".to_string(), Box::new(QwenQuotaExtractor));
        extractors.insert("qwen-code".to_string(), Box::new(QwenQuotaExtractor));
        extractors.insert("qwen-oauth".to_string(), Box::new(QwenQuotaExtractor));
        extractors.insert("dashscope".to_string(), Box::new(QwenQuotaExtractor));

        Self { extractors }
    }

    /// 按提供商专用、兼容响应头、专用错误、兼容错误的顺序提取配额。
    ///
    /// # 参数
    ///
    /// - `provider`: 当前提供商名称。
    /// - `headers`: HTTP 响应头。
    /// - `error`: 可选的提供商错误。
    ///
    /// # 返回
    ///
    /// 首个成功提取的配额元数据；所有提取器均不匹配时返回 `None`。
    pub fn extract(
        &self,
        provider: &str,
        headers: &HeaderMap,
        error: Option<&anyhow::Error>,
    ) -> Option<QuotaMetadata> {
        if let Some(extractor) = self.extractors.get(provider)
            && let Some(quota) = extractor.extract_from_headers(headers)
        {
            tracing::debug!(
                provider,
                remaining = ?quota.rate_limit_remaining,
                "使用提供商专用提取器从响应头解析配额"
            );
            return Some(quota);
        }

        // 部分第三方提供商使用 OpenAI 等兼容响应头，因此继续尝试完整回退链。
        for (name, extractor) in &self.extractors {
            if name != provider
                && let Some(quota) = extractor.extract_from_headers(headers)
            {
                tracing::debug!(
                    provider,
                    extractor = name,
                    remaining = ?quota.rate_limit_remaining,
                    "使用兼容提取器从响应头解析配额"
                );
                return Some(quota);
            }
        }

        if let Some(error) = error {
            if let Some(extractor) = self.extractors.get(provider)
                && let Some(quota) = extractor.extract_from_error(error)
            {
                tracing::debug!(provider, "使用提供商专用提取器从错误解析配额");
                return Some(quota);
            }

            for (name, extractor) in &self.extractors {
                if name != provider
                    && let Some(quota) = extractor.extract_from_error(error)
                {
                    tracing::debug!(provider, extractor = name, "使用兼容提取器从错误解析配额");
                    return Some(quota);
                }
            }
        }

        None
    }

    /// 为指定提供商注册或替换自定义配额提取器。
    ///
    /// # 参数
    ///
    /// - `provider`: 提供商名称。
    /// - `extractor`: 线程安全的自定义配额提取器。
    pub fn register_extractor(&mut self, provider: String, extractor: Box<dyn QuotaExtractor>) {
        self.extractors.insert(provider, extractor);
    }
}

impl Default for UniversalQuotaExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::UniversalQuotaExtractor;
    use reqwest::header::HeaderMap;

    #[test]
    fn uses_provider_specific_and_compatible_header_extractors() {
        let extractor = UniversalQuotaExtractor::new();
        let mut headers = HeaderMap::new();
        headers.insert("X-RateLimit-Remaining", "15".parse().expect("valid header"));

        let quota = extractor
            .extract("openai", &headers, None)
            .expect("provider-specific quota");
        assert_eq!(quota.rate_limit_remaining, Some(15));

        let fallback = extractor
            .extract("custom-provider", &headers, None)
            .expect("compatible quota");
        assert_eq!(fallback.rate_limit_remaining, Some(15));
    }

    #[test]
    fn uses_provider_error_extractor_and_returns_none_without_match() {
        let extractor = UniversalQuotaExtractor::new();
        let headers = HeaderMap::new();
        let error = anyhow::anyhow!("qwen rate limit exceeded");

        let quota = extractor
            .extract("qwen-code", &headers, Some(&error))
            .expect("qwen quota");
        assert_eq!(quota.rate_limit_remaining, Some(0));
        assert_eq!(quota.rate_limit_total, Some(1000));
        assert!(
            extractor
                .extract("unknown-provider", &headers, None)
                .is_none()
        );
    }
}
