// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// 提供给大模型的工具名称、说明与 JSON 参数结构。
///
/// 字段与 AgentScope `ToolSchema` 一一对应。对应 Java: 无（Rust 提供商基础设施；
/// 源自 ZeroClaw `ToolSpec`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// 工具名称。
    pub name: String,
    /// 工具用途说明。
    pub description: String,
    /// JSON Schema 参数定义。
    pub parameters: serde_json::Value,
}
