//! 对应 com.yomahub.liteflow.enums.BooleanConditionTypeEnum（2.11+）：
//! AND/OR 布尔编排条件的类型标记（AndOrCondition 持有）。

/// 布尔条件类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanConditionTypeEnum {
    /// AND：全真为真
    And,
    /// OR：一真即真
    Or,
}
