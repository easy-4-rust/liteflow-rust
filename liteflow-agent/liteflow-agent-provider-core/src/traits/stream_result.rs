// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use super::StreamError;

/// 统一流式操作结果，错误类型固定为 [`StreamError`]。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `StreamResult`）。
pub type StreamResult<T> = std::result::Result<T, StreamError>;
