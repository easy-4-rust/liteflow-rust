// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use super::ChatMessage;
use crate::tool_spec::ToolSpec;

/// 提供商结构化聊天调用的借用请求。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ChatRequest`）。
#[derive(Debug, Clone, Copy)]
pub struct ChatRequest<'a> {
    /// 有序对话消息。
    pub messages: &'a [ChatMessage],
    /// 可选工具定义。
    pub tools: Option<&'a [ToolSpec]>,
}
