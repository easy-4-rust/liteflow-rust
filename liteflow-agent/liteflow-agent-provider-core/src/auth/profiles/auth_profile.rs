// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use super::{AuthProfileKind, TokenSet, profile_id};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 单个模型提供商的 OAuth 或静态 Token 认证配置档案。
///
/// `Debug` 实现主动省略所有凭证字段。对应 Java: 无（Rust 提供商认证基础设施；
/// 源自 ZeroClaw `AuthProfile`）。
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthProfile {
    /// 稳定档案标识。
    pub id: String,
    /// 提供商名称。
    pub provider: String,
    /// 档案名称。
    pub profile_name: String,
    /// 凭证类型。
    pub kind: AuthProfileKind,
    /// 可选账号标识。
    #[serde(default)]
    pub account_id: Option<String>,
    /// 可选工作区标识。
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// OAuth 令牌集合。
    #[serde(default)]
    pub token_set: Option<TokenSet>,
    /// 静态 API Token。
    #[serde(default)]
    pub token: Option<String>,
    /// 提供商扩展元数据。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 创建时间。
    pub created_at: DateTime<Utc>,
    /// 最近更新时间。
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for AuthProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthProfile")
            .field("id", &self.id)
            .field("provider", &self.provider)
            .field("profile_name", &self.profile_name)
            .field("kind", &self.kind)
            .field("workspace_id", &self.workspace_id)
            .field("metadata", &self.metadata)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish_non_exhaustive()
    }
}

impl AuthProfile {
    /// 创建 OAuth 认证档案。
    ///
    /// # 参数
    /// - `provider`: 提供商名称。
    /// - `profile_name`: 档案名称。
    /// - `token_set`: OAuth 令牌集合。
    pub fn new_oauth(provider: &str, profile_name: &str, token_set: TokenSet) -> Self {
        let now = Utc::now();
        Self {
            id: profile_id(provider, profile_name),
            provider: provider.to_string(),
            profile_name: profile_name.to_string(),
            kind: AuthProfileKind::OAuth,
            account_id: None,
            workspace_id: None,
            token_set: Some(token_set),
            token: None,
            metadata: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建静态 Token 认证档案。
    ///
    /// # 参数
    /// - `provider`: 提供商名称。
    /// - `profile_name`: 档案名称。
    /// - `token`: 静态 API Token。
    pub fn new_token(provider: &str, profile_name: &str, token: String) -> Self {
        let now = Utc::now();
        Self {
            id: profile_id(provider, profile_name),
            provider: provider.to_string(),
            profile_name: profile_name.to_string(),
            kind: AuthProfileKind::Token,
            account_id: None,
            workspace_id: None,
            token_set: None,
            token: Some(token),
            metadata: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}
