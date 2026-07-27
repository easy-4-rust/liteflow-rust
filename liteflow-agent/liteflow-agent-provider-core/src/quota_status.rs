// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// 提供商配额与熔断器的统一状态。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `QuotaStatus`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaStatus {
    /// 提供商健康且可用。
    Ok,
    /// 提供商已被限流，但熔断器仍处于关闭状态。
    RateLimited,
    /// 连续失败过多，熔断器已经打开。
    CircuitOpen,
    /// OAuth 配置档案的配额已经耗尽。
    QuotaExhausted,
}
