// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// 大模型请求执行的一次工具调用。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ToolCall`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 提供商生成的工具调用标识。
    pub id: String,
    /// 工具名称。
    pub name: String,
    /// JSON 编码的工具参数。
    pub arguments: String,
}
