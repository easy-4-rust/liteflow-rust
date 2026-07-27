//! 对应 com.yomahub.liteflow.flow.element.condition.BooleanConditionTypeEnum（2.11+）：
//! AND/OR 布尔编排条件的类型标记（AndOrCondition 持有）。

/// AND/OR 布尔编排条件的类型标记。
///
/// 对应 Java:
/// `com.yomahub.liteflow.flow.element.condition.BooleanConditionTypeEnum`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanConditionTypeEnum {
    /// AND：全真为真
    And,
    /// OR：一真即真
    Or,
}
