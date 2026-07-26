//! 对应 com.yomahub.liteflow.enums.ConditionTypeEnum：
//! 规则文件中 condition 的 type 枚举（then/when/switch/if/pre/finally/for/while/iterator/catch）。
//! Java 每个枚举携带 type 与 name 两个字符串字段。

/// 条件类型枚举
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
    /// getType()：对应规则文件中的 type 字符串
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
    /// getName()
    pub fn get_name(&self) -> &'static str {
        self.get_type()
    }
    /// getEnumByCode(code)：按 type 字符串反查枚举
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
