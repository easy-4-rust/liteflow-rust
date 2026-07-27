// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// 对话中的单条角色消息。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ChatMessage`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息角色，例如 system、user、assistant 或 tool。
    pub role: String,
    /// 消息文本内容。
    pub content: String,
}

impl ChatMessage {
    /// 创建系统消息。
    ///
    /// # 参数
    /// - `content`: 系统指令内容。
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    /// 创建用户消息。
    ///
    /// # 参数
    /// - `content`: 用户消息内容。
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    /// 创建助手消息。
    ///
    /// # 参数
    /// - `content`: 助手回复内容。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }

    /// 创建工具结果消息。
    ///
    /// # 参数
    /// - `content`: 工具返回内容。
    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
        }
    }
}
