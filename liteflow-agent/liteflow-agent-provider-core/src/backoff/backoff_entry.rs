// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

/// 记录退避截止时间和对应错误上下文的存储条目。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `BackoffEntry`）。
#[derive(Debug, Clone)]
pub struct BackoffEntry<T> {
    /// 退避结束时刻。
    pub deadline: Instant,
    /// 调用方保存的错误上下文。
    pub error_detail: T,
}
