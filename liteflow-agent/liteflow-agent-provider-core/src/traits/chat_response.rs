// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use super::{TokenUsage, ToolCall};
use crate::quota_metadata::QuotaMetadata;

/// 可同时包含文本、工具调用与推理内容的大模型响应。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ChatResponse`）。
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// 文本内容；仅包含工具调用时可为空。
    pub text: Option<String>,
    /// 大模型请求执行的工具调用。
    pub tool_calls: Vec<ToolCall>,
    /// 提供商报告的 Token 用量。
    pub usage: Option<TokenUsage>,
    /// 思考模型返回的原始推理内容，保留用于后续请求的历史回传。
    pub reasoning_content: Option<String>,
    /// 从响应中提取的配额元数据。
    pub quota_metadata: Option<QuotaMetadata>,
}

impl ChatResponse {
    /// 判断响应是否包含至少一次工具调用。
    ///
    /// # 返回
    /// 包含工具调用时返回 `true`。
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// 返回文本内容；响应没有文本时返回空字符串。
    ///
    /// # 返回
    /// 借用的响应文本。
    pub fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }
}
