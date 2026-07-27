// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0

/// 组合提供商与配置档案名称，生成稳定的认证档案标识。
///
/// # 参数
/// - `provider`: 提供商名称。
/// - `profile_name`: 配置档案名称。
///
/// # 返回
/// 去除两端空白后以冒号连接的标识。
pub fn profile_id(provider: &str, profile_name: &str) -> String {
    format!("{}:{}", provider.trim(), profile_name.trim())
}
