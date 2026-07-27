// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/providers/mod.rs 的 ProviderRuntimeOptions。
// 修改：去掉 CompatibleApiMode 字段（属于 compatible provider 专属）。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! Provider 运行时选项（构造 provider 时传入的配置载体）。

use std::path::PathBuf;

/// Provider 运行时选项。
///
/// 对应 zeroclaw 的 `ProviderRuntimeOptions`，用于在构造 provider 时
/// 传递认证、传输、推理等可选配置。
#[derive(Debug, Clone)]
pub struct ProviderRuntimeOptions {
    /// 认证 profile 覆盖（如指定某个 OAuth profile）。
    pub auth_profile_override: Option<String>,
    /// 自定义 provider API URL。
    pub provider_api_url: Option<String>,
    /// 传输模式（如 "websocket" / "http"）。
    pub provider_transport: Option<String>,
    /// 状态目录（zeroclaw_dir）。
    pub zeroclaw_dir: Option<PathBuf>,
    /// 是否加密存储 secrets。
    pub secrets_encrypt: bool,
    /// 是否启用推理（reasoning）模式。
    pub reasoning_enabled: Option<bool>,
    /// 推理等级（如 "low" / "medium" / "high"）。
    pub reasoning_level: Option<String>,
    /// max_tokens 覆盖。
    pub max_tokens_override: Option<u32>,
    /// 是否支持视觉输入。
    pub model_support_vision: Option<bool>,
}

impl Default for ProviderRuntimeOptions {
    fn default() -> Self {
        Self {
            auth_profile_override: None,
            provider_api_url: None,
            provider_transport: None,
            zeroclaw_dir: None,
            secrets_encrypt: true,
            reasoning_enabled: None,
            reasoning_level: None,
            max_tokens_override: None,
            model_support_vision: None,
        }
    }
}
