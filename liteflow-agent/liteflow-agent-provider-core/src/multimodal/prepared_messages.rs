// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::traits::ChatMessage;

/// 图片引用规范化完成后的提供商消息集合。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `PreparedMessages`）。
#[derive(Debug, Clone)]
pub struct PreparedMessages {
    /// 已规范化的消息。
    pub messages: Vec<ChatMessage>,
    /// 消息中是否包含图片。
    pub contains_images: bool,
}
