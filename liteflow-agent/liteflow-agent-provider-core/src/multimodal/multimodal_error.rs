// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 多模态图片解析、读取、校验与下载错误。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `MultimodalError`）。
#[derive(Debug, thiserror::Error)]
pub enum MultimodalError {
    /// 图片数量超过配置上限。
    #[error("multimodal image limit exceeded: max_images={max_images}, found={found}")]
    TooManyImages {
        /// 最大图片数。
        max_images: usize,
        /// 实际图片数。
        found: usize,
    },
    /// 图片大小超过配置上限。
    #[error(
        "multimodal image size limit exceeded for '{input}': {size_bytes} bytes > {max_bytes} bytes"
    )]
    ImageTooLarge {
        /// 原始图片引用。
        input: String,
        /// 实际字节数。
        size_bytes: usize,
        /// 最大字节数。
        max_bytes: usize,
    },
    /// 图片 MIME 类型不在允许列表中。
    #[error("multimodal image MIME type is not allowed for '{input}': {mime}")]
    UnsupportedMime {
        /// 原始图片引用。
        input: String,
        /// 检测到的 MIME 类型。
        mime: String,
    },
    /// 配置禁止下载远程图片。
    #[error("multimodal remote image fetch is disabled for '{input}'")]
    RemoteFetchDisabled {
        /// 远程图片 URL。
        input: String,
    },
    /// 本地图片不存在或不可读。
    #[error("multimodal image source not found or unreadable: '{input}'")]
    ImageSourceNotFound {
        /// 本地图片路径。
        input: String,
    },
    /// 图片标记或 Data URI 格式无效。
    #[error("invalid multimodal image marker '{input}': {reason}")]
    InvalidMarker {
        /// 原始图片标记。
        input: String,
        /// 无效原因。
        reason: String,
    },
    /// 远程图片下载失败。
    #[error("failed to download remote image '{input}': {reason}")]
    RemoteFetchFailed {
        /// 远程图片 URL。
        input: String,
        /// 失败原因。
        reason: String,
    },
    /// 本地图片读取失败。
    #[error("failed to read local image '{input}': {reason}")]
    LocalReadFailed {
        /// 本地图片路径。
        input: String,
        /// 失败原因。
        reason: String,
    },
}
