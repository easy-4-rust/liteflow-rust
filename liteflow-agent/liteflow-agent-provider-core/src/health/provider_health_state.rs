// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 单个提供商的连续失败次数与最近错误状态。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ProviderHealthState`）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderHealthState {
    /// 连续失败次数。
    pub failure_count: u32,
    /// 最近一次错误消息。
    pub last_error: Option<String>,
}
