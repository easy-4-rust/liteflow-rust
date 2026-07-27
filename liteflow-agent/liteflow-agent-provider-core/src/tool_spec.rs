// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/tools/traits.rs，仅保留 ToolSpec / ToolResult 类型。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

use serde::{Deserialize, Serialize};

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Description of a tool for the LLM.
///
/// 对应 agentscope 的 `ToolSchema`，字段一一对应，
/// 用于在 zeroclaw `Provider` trait 体系中传递工具定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
