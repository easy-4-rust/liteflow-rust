//! Chain 查询结果。

use serde::{Deserialize, Serialize};

/// 保存一条 Chain 的 id、路由、命名空间和执行体。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.vo.ChainVO`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainVO {
    /// Chain id。
    pub chain_id: String,
    /// 决策路由 EL。
    pub route: Option<String>,
    /// 命名空间。
    pub namespace: Option<String>,
    /// Chain 执行体 EL。
    pub body: String,
}
