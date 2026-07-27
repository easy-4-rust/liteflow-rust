// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/providers/quota_types.rs。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! Shared types for quota and rate limit tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Quota metadata extracted from provider responses (HTTP headers or errors).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaMetadata {
    /// Number of requests remaining in current quota window
    pub rate_limit_remaining: Option<u64>,
    /// Timestamp when the rate limit resets (UTC)
    pub rate_limit_reset_at: Option<DateTime<Utc>>,
    /// Number of seconds to wait before retry (from Retry-After header)
    pub retry_after_seconds: Option<u64>,
    /// Maximum requests allowed in quota window (if available)
    pub rate_limit_total: Option<u64>,
}

/// Status of a provider's quota and circuit breaker state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaStatus {
    /// Provider is healthy and available
    Ok,
    /// Provider is rate-limited but circuit is still closed
    RateLimited,
    /// Circuit breaker is open (too many failures)
    CircuitOpen,
    /// OAuth profile quota exhausted
    QuotaExhausted,
}

/// Per-provider quota information combining health state and OAuth profile metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderQuotaInfo {
    pub provider: String,
    pub status: QuotaStatus,
    pub failure_count: u32,
    pub last_error: Option<String>,
    pub retry_after_seconds: Option<u64>,
    pub circuit_resets_at: Option<DateTime<Utc>>,
    pub profiles: Vec<ProfileQuotaInfo>,
}

/// Per-OAuth-profile quota information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileQuotaInfo {
    pub profile_name: String,
    pub status: QuotaStatus,
    pub rate_limit_remaining: Option<u64>,
    pub rate_limit_reset_at: Option<DateTime<Utc>>,
    pub rate_limit_total: Option<u64>,
    /// Account identifier (email, workspace ID, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// When the OAuth token / subscription expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Plan type (free, pro, enterprise) if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
}

/// Summary of all providers' quota status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSummary {
    pub timestamp: DateTime<Utc>,
    pub providers: Vec<ProviderQuotaInfo>,
}

impl QuotaSummary {
    /// Get available (healthy) providers
    pub fn available_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|p| p.status == QuotaStatus::Ok)
            .map(|p| p.provider.as_str())
            .collect()
    }

    /// Get rate-limited providers
    pub fn rate_limited_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|p| {
                p.status == QuotaStatus::RateLimited || p.status == QuotaStatus::QuotaExhausted
            })
            .map(|p| p.provider.as_str())
            .collect()
    }

    /// Get circuit-open providers
    pub fn circuit_open_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|p| p.status == QuotaStatus::CircuitOpen)
            .map(|p| p.provider.as_str())
            .collect()
    }
}
