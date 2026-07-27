//! 脚本节点查询结果。

use serde::{Deserialize, Serialize};

/// 保存脚本节点元数据和脚本文本。
///
/// 对应 Java: `com.yomahub.liteflow.parser.sql.read.vo.ScriptVO`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptVO {
    /// 节点 id。
    pub node_id: String,
    /// 节点类型。
    #[serde(rename = "type")]
    pub script_type: String,
    /// 节点显示名称。
    pub name: Option<String>,
    /// 脚本语言。
    pub language: Option<String>,
    /// 是否启用。
    pub enable: Option<bool>,
    /// 脚本文本。
    pub script: String,
}
