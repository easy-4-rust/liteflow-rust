// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use crate::profile_quota_info::ProfileQuotaInfo;
use crate::quota_status::QuotaStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 汇总提供商健康状态、熔断状态与 OAuth 配置档案的配额信息。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ProviderQuotaInfo`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderQuotaInfo {
    /// 提供商名称。
    pub provider: String,
    /// 当前提供商状态。
    pub status: QuotaStatus,
    /// 当前累计失败次数。
    pub failure_count: u32,
    /// 最近一次错误消息。
    pub last_error: Option<String>,
    /// 再次请求前应等待的秒数。
    pub retry_after_seconds: Option<u64>,
    /// 熔断器预计恢复的 UTC 时间。
    pub circuit_resets_at: Option<DateTime<Utc>>,
    /// 该提供商下各 OAuth 配置档案的配额信息。
    pub profiles: Vec<ProfileQuotaInfo>,
}
