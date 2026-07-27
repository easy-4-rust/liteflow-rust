// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::tool_spec::ToolSpec;
use std::fmt::Write;

/// 构造提示词引导式工具调用协议文本。
///
/// # 参数
/// - `tools`: 要暴露给模型的统一工具定义。
///
/// # 返回
/// 包含 XML 调用协议、工具名称、说明和 JSON 参数结构的提示词。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw
/// `build_tool_instructions_text`）。
pub fn build_tool_instructions_text(tools: &[ToolSpec]) -> String {
    let mut instructions = String::new();
    instructions.push_str("## Tool Use Protocol\n\n");
    instructions.push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
    instructions.push_str("<tool_call>\n");
    instructions.push_str(r#"{"name": "tool_name", "arguments": {"param": "value"}}"#);
    instructions.push_str("\n</tool_call>\n\n");
    instructions.push_str("You may use multiple tool calls in a single response. ");
    instructions.push_str("After tool execution, results appear in <tool_result> tags. ");
    instructions
        .push_str("Continue reasoning with the results until you can give a final answer.\n\n");
    instructions.push_str("### Available Tools\n\n");

    for tool in tools {
        writeln!(&mut instructions, "**{}**: {}", tool.name, tool.description)
            .expect("writing to String cannot fail");
        let parameters =
            serde_json::to_string(&tool.parameters).unwrap_or_else(|_| "{}".to_string());
        writeln!(&mut instructions, "Parameters: `{parameters}`")
            .expect("writing to String cannot fail");
        instructions.push('\n');
    }

    instructions
}
