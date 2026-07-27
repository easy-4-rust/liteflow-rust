// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::quota_status::QuotaStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 单个 OAuth 配置档案的配额、账号及订阅信息。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ProfileQuotaInfo`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileQuotaInfo {
    /// 配置档案名称。
    pub profile_name: String,
    /// 当前配额状态。
    pub status: QuotaStatus,
    /// 当前配额窗口内剩余的请求次数。
    pub rate_limit_remaining: Option<u64>,
    /// 当前限流窗口的 UTC 重置时间。
    pub rate_limit_reset_at: Option<DateTime<Utc>>,
    /// 当前配额窗口允许的最大请求次数。
    pub rate_limit_total: Option<u64>,
    /// 账号标识，例如邮箱或工作区 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// OAuth 令牌或订阅的 UTC 到期时间。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_expires_at: Option<DateTime<Utc>>,
    /// 已知的套餐类型，例如 free、pro 或 enterprise。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
}
