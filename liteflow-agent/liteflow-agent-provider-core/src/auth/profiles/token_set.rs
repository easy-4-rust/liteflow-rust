// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// OAuth 认证产生的访问令牌、刷新令牌及有效期集合。
///
/// 对应 Java: 无（Rust 提供商认证基础设施；源自 ZeroClaw `TokenSet`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    /// 访问令牌。
    pub access_token: String,
    /// 可选刷新令牌。
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// 可选 OpenID Connect ID Token。
    #[serde(default)]
    pub id_token: Option<String>,
    /// 访问令牌 UTC 到期时间。
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// 令牌类型，例如 Bearer。
    #[serde(default)]
    pub token_type: Option<String>,
    /// OAuth 授权范围。
    #[serde(default)]
    pub scope: Option<String>,
}

impl TokenSet {
    /// 判断令牌是否将在指定提前量内过期。
    ///
    /// # 参数
    /// - `skew`: 到期前的刷新提前量。
    ///
    /// # 返回
    /// 已设置到期时间且到期时刻不晚于当前时刻加提前量时返回 `true`。
    pub fn is_expiring_within(&self, skew: Duration) -> bool {
        match self.expires_at {
            Some(expires_at) => {
                let now_plus_skew =
                    Utc::now() + chrono::Duration::from_std(skew).unwrap_or_default();
                expires_at <= now_plus_skew
            }
            None => false,
        }
    }
}
