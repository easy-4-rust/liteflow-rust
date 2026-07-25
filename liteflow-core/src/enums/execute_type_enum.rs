//! 对应 com.yomahub.liteflow.enums.ExecuteTypeEnum：
//! CmpStep 执行粒度的类型标记（链 / 条件 / 节点）。

/// 执行类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecuteTypeEnum {
    /// 链路级
    Chain,
    /// 条件级
    Condition,
    /// 节点级
    Node,
}
