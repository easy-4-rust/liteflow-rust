// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// OpenAI 兼容 API 密钥的发送方式。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `AuthStyle`）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AuthStyle {
    /// 使用 OpenAI 标准 `Authorization: Bearer <key>`。
    #[default]
    Bearer,
    /// 使用服务自定义的认证请求头。
    Custom,
}
