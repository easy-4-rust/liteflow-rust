// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 不同模型提供商所需的工具定义载荷。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ToolsPayload`）。
#[derive(Debug, Clone)]
pub enum ToolsPayload {
    /// Gemini `functionDeclarations` 格式。
    Gemini {
        /// 函数声明列表。
        function_declarations: Vec<serde_json::Value>,
    },
    /// Anthropic Messages API 的 `input_schema` 工具格式。
    Anthropic {
        /// 工具定义列表。
        tools: Vec<serde_json::Value>,
    },
    /// OpenAI Chat Completions 的 function 工具格式。
    OpenAI {
        /// 工具定义列表。
        tools: Vec<serde_json::Value>,
    },
    /// 将工具说明注入系统提示词的兼容回退格式。
    PromptGuided {
        /// 格式化后的工具说明。
        instructions: String,
    },
}
