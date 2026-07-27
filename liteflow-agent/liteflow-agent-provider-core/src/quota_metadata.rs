// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 从提供商 HTTP 响应头或错误中提取的配额元数据。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `QuotaMetadata`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaMetadata {
    /// 当前配额窗口内剩余的请求次数。
    pub rate_limit_remaining: Option<u64>,
    /// 当前限流窗口的 UTC 重置时间。
    pub rate_limit_reset_at: Option<DateTime<Utc>>,
    /// 再次请求前应等待的秒数，通常来自 `Retry-After`。
    pub retry_after_seconds: Option<u64>,
    /// 当前配额窗口允许的最大请求次数。
    pub rate_limit_total: Option<u64>,
}
