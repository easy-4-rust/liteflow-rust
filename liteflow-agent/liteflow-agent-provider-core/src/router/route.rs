// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 将任务提示映射到提供商与模型组合的路由条目。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `Route`）。
#[derive(Debug, Clone)]
pub struct Route {
    /// 目标提供商名称。
    pub provider_name: String,
    /// 目标模型名称。
    pub model: String,
}
