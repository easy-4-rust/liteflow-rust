// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 单次大模型 API 响应报告的原始 Token 用量。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `TokenUsage`）。
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// 输入 Token 数。
    pub input_tokens: Option<u64>,
    /// 输出 Token 数。
    pub output_tokens: Option<u64>,
}
