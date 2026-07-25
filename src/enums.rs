//! 对应 liteflow-core enums 包。

/// ConditionTypeEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionTypeEnum {
    Then,
    When,
    If,
    Switch,
    For,
    While,
    Iterator,
    Catch,
    Pre,
    Finally,
    AndOr,
    Not,
    Retry,
    Timeout,
}

/// ParallelStrategyEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelStrategyEnum {
    All,
    Any,
    Percentage,
    Specify,
}

/// 对应 condition/BooleanConditionTypeEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanConditionTypeEnum {
    And,
    Or,
}

/// CmpStepTypeEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpStepTypeEnum {
    Node,
    Condition,
}

/// NodeTypeEnum（脚本/普通节点的类型标记，脚本节点为路线图保留）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTypeEnum {
    Common,
    Boolean,
    Switch,
    For,
    Iterator,
    Script,
}
