// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 多模态图片数量、大小与远程访问配置。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `MultimodalConfig`）。
#[derive(Debug, Clone)]
pub struct MultimodalConfig {
    /// 每个请求接受的最大图片数量。
    pub max_images: usize,
    /// Base64 编码前的最大图片大小，单位为 MiB。
    pub max_image_size_mb: usize,
    /// 是否允许获取 HTTP/HTTPS 远程图片；默认关闭。
    pub allow_remote_fetch: bool,
}

impl Default for MultimodalConfig {
    fn default() -> Self {
        Self {
            max_images: 4,
            max_image_size_mb: 5,
            allow_remote_fetch: false,
        }
    }
}

impl MultimodalConfig {
    /// 返回钳制到安全范围的图片数量与大小限制。
    ///
    /// # 返回
    /// `(max_images, max_image_size_mb)`，分别限制在 1..=16 与 1..=20。
    pub fn effective_limits(&self) -> (usize, usize) {
        (
            self.max_images.clamp(1, 16),
            self.max_image_size_mb.clamp(1, 20),
        )
    }
}
