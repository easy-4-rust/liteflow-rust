// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 提供商流式聊天的控制选项。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `StreamOptions`）。
#[derive(Debug, Clone, Copy, Default)]
pub struct StreamOptions {
    /// 是否启用流式响应。
    pub enabled: bool,
    /// 是否为每个响应段估算 Token 数。
    pub count_tokens: bool,
}

impl StreamOptions {
    /// 创建指定启用状态的流式选项。
    ///
    /// # 参数
    /// - `enabled`: 是否启用流式响应。
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            count_tokens: false,
        }
    }

    /// 启用响应段 Token 计数。
    ///
    /// # 返回
    /// 已启用 Token 计数的选项。
    pub fn with_token_count(mut self) -> Self {
        self.count_tokens = true;
        self
    }
}
