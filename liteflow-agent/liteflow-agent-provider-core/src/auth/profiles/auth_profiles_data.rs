// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use super::AuthProfile;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const CURRENT_SCHEMA_VERSION: u32 = 1;

/// 认证配置档案存储的完整内存快照。
///
/// 对应 Java: 无（Rust 提供商认证基础设施；源自 ZeroClaw `AuthProfilesData`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProfilesData {
    /// 文件结构版本。
    pub schema_version: u32,
    /// 快照最近更新时间。
    pub updated_at: DateTime<Utc>,
    /// 每个提供商当前启用的档案标识。
    pub active_profiles: BTreeMap<String, String>,
    /// 按档案标识索引的完整档案。
    pub profiles: BTreeMap<String, AuthProfile>,
}

impl Default for AuthProfilesData {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            updated_at: Utc::now(),
            active_profiles: BTreeMap::new(),
            profiles: BTreeMap::new(),
        }
    }
}
