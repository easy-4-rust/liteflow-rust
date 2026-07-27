// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 流式响应中的一段增量内容。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `StreamChunk`）。
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// 本段文本增量。
    pub delta: String,
    /// 是否为流的最后一段。
    pub is_final: bool,
    /// 本段的近似 Token 数。
    pub token_count: usize,
}

impl StreamChunk {
    /// 创建非结束文本增量。
    ///
    /// # 参数
    /// - `text`: 本段文本。
    pub fn delta(text: impl Into<String>) -> Self {
        Self {
            delta: text.into(),
            is_final: false,
            token_count: 0,
        }
    }

    /// 创建不含文本的结束标记。
    pub fn final_chunk() -> Self {
        Self {
            delta: String::new(),
            is_final: true,
            token_count: 0,
        }
    }

    /// 创建携带错误消息的结束段。
    ///
    /// # 参数
    /// - `message`: 错误消息。
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            delta: message.into(),
            is_final: true,
            token_count: 0,
        }
    }

    /// 按约四个字符一个 Token 估算本段用量。
    ///
    /// # 返回
    /// 写入估算 Token 数后的流式响应段。
    pub fn with_token_estimate(mut self) -> Self {
        self.token_count = self.delta.len().div_ceil(4);
        self
    }
}
