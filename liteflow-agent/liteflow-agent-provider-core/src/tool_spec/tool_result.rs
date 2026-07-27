// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// 一次工具执行的成功状态、输出与可选错误。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ToolResult`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 工具是否成功执行。
    pub success: bool,
    /// 工具标准输出。
    pub output: String,
    /// 可选错误说明。
    pub error: Option<String>,
}
