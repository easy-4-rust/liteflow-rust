// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 提供商支持的模型能力声明，用于选择工具调用与请求适配策略。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `ProviderCapabilities`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderCapabilities {
    /// 是否支持 API 原生工具调用；否则工具定义必须注入系统提示词。
    pub native_tool_calling: bool,
    /// 是否支持图片输入。
    pub vision: bool,
}
