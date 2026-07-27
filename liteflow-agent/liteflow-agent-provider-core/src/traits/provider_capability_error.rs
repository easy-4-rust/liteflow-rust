// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 请求的模型能力不受提供商支持时返回的结构化错误。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ProviderCapabilityError`）。
#[derive(Debug, Clone, thiserror::Error)]
#[error("provider_capability_error provider={provider} capability={capability} message={message}")]
pub struct ProviderCapabilityError {
    /// 提供商名称。
    pub provider: String,
    /// 不受支持的能力名称。
    pub capability: String,
    /// 面向调用方的错误说明。
    pub message: String,
}
