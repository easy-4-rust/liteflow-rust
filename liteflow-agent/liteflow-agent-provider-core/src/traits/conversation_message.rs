// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use super::{ChatMessage, ToolCall, ToolResultMessage};
use serde::{Deserialize, Serialize};

/// 支持普通消息、助手工具调用与工具结果的多轮会话消息。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ConversationMessage`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConversationMessage {
    /// system、user 或 assistant 普通消息。
    Chat(ChatMessage),
    /// 助手产生的工具调用，按原始顺序保存。
    AssistantToolCalls {
        /// 工具调用之外的可选文本。
        text: Option<String>,
        /// 助手产生的工具调用。
        tool_calls: Vec<ToolCall>,
        /// 思考模型的原始推理内容，用于无损历史回传。
        reasoning_content: Option<String>,
    },
    /// 工具执行结果。
    ToolResults(Vec<ToolResultMessage>),
}
