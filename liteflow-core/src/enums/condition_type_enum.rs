//! 规则文件中 Condition 的类型枚举。
//!
//! 包含 then/when/switch/if/pre/finally/for/while/iterator/catch 等类型。
//! Java 每个枚举携带 type 与 name 两个字符串字段。

/// 标识一个 Condition 的构建与执行类型。
///
/// Java 枚举的 `type` 和 `name` 初始值相同；Rust 使用不可变枚举保持该不变量，
/// 因而不提供会破坏枚举身份的 setter。`Retry` 与 `Timeout` 是 Rust 对修饰条件
/// 对象的显式类型扩展。对应 Java:
/// `com.yomahub.liteflow.enums.ConditionTypeEnum`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionTypeEnum {
    Then,
    When,
    Switch,
    If,
    Pre,
    Finally,
    For,
    While,
    Iterator,
    Catch,
    /// 布尔 AND/OR 编排。
    #[serde(rename = "and_or_opt")]
    AndOr,
    /// 布尔 NOT 编排。
    #[serde(rename = "not_opt")]
    Not,
    /// 含未实现变量、不可执行的抽象 Chain。
    Abstract,
    /// 子 Chain 的 bind 包装条件。
    ChainBindWrapper,
    /// Rust 端扩展：重试修饰（对应 RetryCondition）
    Retry,
    /// Rust 端扩展：超时修饰（对应 TimeoutCondition）
    Timeout,
}

impl ConditionTypeEnum {
    /// 返回规则文件使用的 Condition 类型码。
    ///
    /// 返回值对应 Java: `ConditionTypeEnum#getType`。
    #[must_use]
    pub fn get_type(&self) -> &'static str {
        match self {
            Self::Then => "then",
            Self::When => "when",
            Self::Switch => "switch",
            Self::If => "if",
            Self::Pre => "pre",
            Self::Finally => "finally",
            Self::For => "for",
            Self::While => "while",
            Self::Iterator => "iterator",
            Self::Catch => "catch",
            Self::AndOr => "and_or_opt",
            Self::Not => "not_opt",
            Self::Abstract => "abstract",
            Self::ChainBindWrapper => "chain_bind_wrapper",
            Self::Retry => "retry",
            Self::Timeout => "timeout",
        }
    }
    /// 返回 Condition 类型名称。
    ///
    /// Java 当前 `name` 与 `type` 相同；Rust 从同一不可变映射返回。
    /// 对应 Java: `ConditionTypeEnum#getName`。
    #[must_use]
    pub fn get_name(&self) -> &'static str {
        self.get_type()
    }

    /// 按类型码反查枚举。
    ///
    /// 参数 `code` 对应 Java 同名参数；未匹配时返回 `None`，对应 Java 的 null。
    /// 对应 Java: `ConditionTypeEnum#getEnumByCode`。
    #[must_use]
    pub fn get_enum_by_code(code: &str) -> Option<Self> {
        [
            Self::Then,
            Self::When,
            Self::Switch,
            Self::If,
            Self::Pre,
            Self::Finally,
            Self::For,
            Self::While,
            Self::Iterator,
            Self::Catch,
            Self::AndOr,
            Self::Not,
            Self::Abstract,
            Self::ChainBindWrapper,
            Self::Retry,
            Self::Timeout,
        ]
        .into_iter()
        .find(|e| e.get_type() == code)
    }
}
