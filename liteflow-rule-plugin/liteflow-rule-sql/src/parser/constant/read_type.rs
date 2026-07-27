//! SQL 读取对象类型。

use serde::{Deserialize, Serialize};

/// 区分 Chain、脚本与节点实例编号三类 SQL 数据。
///
/// 对应 Java: `com.yomahub.liteflow.parser.constant.ReadType`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadType {
    /// Chain 规则。
    Chain,
    /// 脚本节点。
    Script,
    /// 节点实例编号持久化记录。
    InstanceId,
}
