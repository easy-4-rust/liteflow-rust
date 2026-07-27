// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/config/schema.rs 中的代理客户端构建逻辑，
// 简化为构建时一次性配置（去掉了 zeroclaw 的全局运行时代理状态热更新）。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! 简化版代理客户端构建。
//!
//! zeroclaw 的 provider 大量调用 `build_runtime_proxy_client_with_timeouts`，
//! 本模块提供等价能力，但去掉了全局 `OnceLock<RwLock<...>>` 运行时热更新，
//! 改为构建时一次性配置（通过 `ProxyConfig` 显式传入）。

use std::time::Duration;

use reqwest::Client;

/// 代理配置（对应 zeroclaw 的 ProxyConfig，简化版）。
#[derive(Debug, Clone, Default)]
pub struct ProxyConfig {
    /// HTTP/HTTPS/SOCKS 代理 URL（如 "http://127.0.0.1:7890"）。
    pub proxy_url: Option<String>,
    /// NO_PROXY 旁路列表。
    pub no_proxy: Vec<String>,
}

impl ProxyConfig {
    /// 创建一个不使用代理的默认配置。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置代理 URL。
    #[must_use]
    pub fn with_proxy(mut self, url: impl Into<String>) -> Self {
        self.proxy_url = Some(url.into());
        self
    }
}

/// 构建带超时的 reqwest 客户端（可选代理）。
///
/// 对应 zeroclaw `build_runtime_proxy_client_with_timeouts(service_key, timeout, connect_timeout)`，
/// 但去掉 service_key 与全局缓存，代理通过 `proxy` 参数显式传入。
#[must_use]
pub fn build_client_with_timeouts(
    timeout_secs: u64,
    connect_timeout_secs: u64,
    proxy: &ProxyConfig,
) -> Client {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(connect_timeout_secs));

    if let Some(url) = &proxy.proxy_url {
        if let Ok(p) = reqwest::Proxy::all(url) {
            builder = builder.proxy(p);
        }
    }

    builder.build().unwrap_or_else(|_| Client::new())
}

/// 构建无超时限制的 reqwest 客户端（可选代理）。
#[must_use]
pub fn build_client(proxy: &ProxyConfig) -> Client {
    let mut builder = Client::builder();
    if let Some(url) = &proxy.proxy_url {
        if let Ok(p) = reqwest::Proxy::all(url) {
            builder = builder.proxy(p);
        }
    }
    builder.build().unwrap_or_else(|_| Client::new())
}
