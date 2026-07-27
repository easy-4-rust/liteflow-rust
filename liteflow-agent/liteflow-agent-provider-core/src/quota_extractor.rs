// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::quota_metadata::QuotaMetadata;
use reqwest::header::HeaderMap;

/// 从不同提供商的响应头或错误消息中提取统一配额元数据。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `QuotaExtractor`）。
pub trait QuotaExtractor: Send + Sync {
    /// 从 HTTP 响应头提取配额信息。
    ///
    /// # 参数
    ///
    /// - `headers`: 提供商返回的 HTTP 响应头。
    ///
    /// # 返回
    ///
    /// 响应头包含可识别配额字段时返回统一元数据，否则返回 `None`。
    fn extract_from_headers(&self, headers: &HeaderMap) -> Option<QuotaMetadata>;

    /// 从错误消息提取配额信息，作为响应头不可用时的回退。
    ///
    /// # 参数
    ///
    /// - `error`: 提供商调用产生的错误。
    ///
    /// # 返回
    ///
    /// 错误包含可识别限流语义时返回统一元数据，否则返回 `None`。
    fn extract_from_error(&self, error: &anyhow::Error) -> Option<QuotaMetadata>;
}
