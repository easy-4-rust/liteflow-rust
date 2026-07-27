// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 提供商流式响应处理过程中可能发生的错误。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `StreamError`）。
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// HTTP 调用错误。
    #[error("HTTP error: {0}")]
    Http(reqwest::Error),
    /// JSON 解析错误。
    #[error("JSON parse error: {0}")]
    Json(serde_json::Error),
    /// 无效的 SSE 数据格式。
    #[error("Invalid SSE format: {0}")]
    InvalidSse(String),
    /// 提供商返回的业务错误。
    #[error("Provider error: {0}")]
    Provider(String),
    /// 流读写错误。
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
