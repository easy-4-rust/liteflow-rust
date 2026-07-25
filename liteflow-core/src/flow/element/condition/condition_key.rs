//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.ConditionKey
//!
//! Java 中 ConditionKey 是一个纯常量接口，定义了 Condition.executableGroup
//! （Map<String, List<Executable>>）的所有分组键：每个 Condition 子类以这些
//! 字符串为键，把不同角色的可执行元素（IF 节点、true/false 分支、DO/BREAK、
//! SWITCH 目标列表、PRE/FINALLY 列表等）分桶存放。
//!
//! Rust 端架构差异说明：
//! Rust 没有采用「字符串键 → 列表」的通用 Map，而是让每个 Condition 结构体
//! 用带类型的字段直接持有这些角色（语义一一对应）：
//!
//! | Java 键                | Rust 承载位置                                        |
//! |------------------------|-----------------------------------------------------|
//! | DEFAULT_KEY            | ThenCondition.executable_list / WhenCondition.executable_list |
//! | FOR_KEY                | ForCondition.for_node                                |
//! | IF_KEY                 | IfCondition.if_item                                  |
//! | IF_TRUE_CASE_KEY       | IfCondition.true_case                                |
//! | IF_FALSE_CASE_KEY      | IfCondition.false_case                               |
//! | ITERATOR_KEY           | IteratorCondition.iterator_node                      |
//! | DO_KEY                 | *Condition.do_executor / CatchCondition.do_item      |
//! | BREAK_KEY              | *Condition.break_item（FOR/WHILE/ITERATOR）          |
//! | SWITCH_KEY             | SwitchCondition.switch_node                          |
//! | SWITCH_TARGET_KEY      | SwitchCondition.target_list                          |
//! | SWITCH_DEFAULT_KEY     | SwitchCondition.default_executor                     |
//! | PRE_KEY                | ThenCondition.pre_list                               |
//! | FINALLY_KEY            | ThenCondition.finally_list                           |
//! | WHILE_KEY              | WhileCondition.while_item                            |
//! | CATCH_KEY              | CatchCondition.catch_item                            |
//!
//! 另外 Java 2.16 起 Condition 上还有 bindData（data/bind 索引），
//! Rust 端由 BindWrapperCondition + Frame::push_bind/find_bind 承载。
//!
//! 本类型把 15 个键建模为枚举，保留与 Java 常量一致的字符串值（as_str），
//! 并提供相等/哈希语义（derive PartialEq/Eq/Hash），可作为 HashMap 键使用，
//! 以便需要「按键分组」语义的调用方（如测试、诊断工具）保持与 Java 对齐。

/// Condition 可执行元素分组键（对应 Java ConditionKey 接口的 15 个常量）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionKey {
    /// DEFAULT_KEY：主可执行列表（THEN/WHEN 主体）
    Default,
    /// FOR_KEY：FOR 循环次数节点
    For,
    /// IF_KEY：IF 条件节点
    If,
    /// IF_TRUE_CASE_KEY：IF 为 true 的分支
    IfTrueCase,
    /// IF_FALSE_CASE_KEY：IF 为 false 的分支
    IfFalseCase,
    /// ITERATOR_KEY：ITERATOR 迭代节点
    Iterator,
    /// DO_KEY：循环体 / CATCH 的 DO 分支
    Do,
    /// BREAK_KEY：循环 BREAK 节点
    Break,
    /// SWITCH_KEY：SWITCH 选择节点
    Switch,
    /// SWITCH_TARGET_KEY：SWITCH 候选目标列表
    SwitchTarget,
    /// SWITCH_DEFAULT_KEY：SWITCH 默认目标
    SwitchDefault,
    /// PRE_KEY：前置 Condition 列表
    Pre,
    /// FINALLY_KEY：后置 Condition 列表
    Finally,
    /// WHILE_KEY：WHILE 条件节点
    While,
    /// CATCH_KEY：CATCH 被捕获执行体
    Catch,
}

impl ConditionKey {
    /// 全部键（对应 Java 接口的常量全集）
    pub const ALL: [ConditionKey; 15] = [
        ConditionKey::Default,
        ConditionKey::For,
        ConditionKey::If,
        ConditionKey::IfTrueCase,
        ConditionKey::IfFalseCase,
        ConditionKey::Iterator,
        ConditionKey::Do,
        ConditionKey::Break,
        ConditionKey::Switch,
        ConditionKey::SwitchTarget,
        ConditionKey::SwitchDefault,
        ConditionKey::Pre,
        ConditionKey::Finally,
        ConditionKey::While,
        ConditionKey::Catch,
    ];

    /// 与 Java ConditionKey 常量一致的字符串值
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionKey::Default => "DEFAULT_KEY",
            ConditionKey::For => "FOR_KEY",
            ConditionKey::If => "IF_KEY",
            ConditionKey::IfTrueCase => "IF_TRUE_CASE_KEY",
            ConditionKey::IfFalseCase => "IF_FALSE_CASE_KEY",
            ConditionKey::Iterator => "ITERATOR_KEY",
            ConditionKey::Do => "DO_KEY",
            ConditionKey::Break => "BREAK_KEY",
            ConditionKey::Switch => "SWITCH_KEY",
            ConditionKey::SwitchTarget => "SWITCH_TARGET_KEY",
            ConditionKey::SwitchDefault => "SWITCH_DEFAULT_KEY",
            ConditionKey::Pre => "PRE_KEY",
            ConditionKey::Finally => "FINALLY_KEY",
            ConditionKey::While => "WHILE_KEY",
            ConditionKey::Catch => "CATCH_KEY",
        }
    }

    /// 按 Java 常量字符串反查键（对应 Map 查找语义）
    pub fn from_key(s: &str) -> Option<ConditionKey> {
        ConditionKey::ALL.iter().copied().find(|k| k.as_str() == s)
    }
}
