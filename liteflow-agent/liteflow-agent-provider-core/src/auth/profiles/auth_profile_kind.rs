// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// 认证配置档案的凭证类型。
///
/// 对应 Java: 无（Rust 提供商认证基础设施；源自 ZeroClaw `AuthProfileKind`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthProfileKind {
    /// OAuth 访问令牌与可选刷新令牌。
    OAuth,
    /// 长期静态 API Token。
    Token,
}
