// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// 回传给大模型的一次工具执行结果。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ToolResultMessage`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMessage {
    /// 对应工具调用的标识。
    pub tool_call_id: String,
    /// 工具执行结果文本。
    pub content: String,
}
