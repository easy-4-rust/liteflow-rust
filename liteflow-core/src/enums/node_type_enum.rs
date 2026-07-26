//! 对应 com.yomahub.liteflow.enums.NodeTypeEnum：
//! 节点类型枚举（普通/选择/条件/循环/跳出/迭代 + 脚本系）。
//! Java 每个枚举携带 code、name、isScript 及映射的组件类；
//! Rust 端组件类映射由返回值类型约定表达（见 docs/语义迁移对照表.md 第三章）。

/// 节点类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeTypeEnum {
    Common,
    Switch,
    /// Java v2.16 的 BOOLEAN 节点。
    Boolean,
    /// Rust 历史兼容别名；新规则应使用 Boolean。
    If,
    For,
    /// Rust 历史兼容别名；WHILE 条件同样由 Boolean 节点承担。
    While,
    /// Rust 历史兼容别名；BREAK 条件同样由 Boolean 节点承担。
    Break,
    Iterator,
    Script,
    SwitchScript,
    /// Java v2.16 的 BOOLEAN_SCRIPT 节点。
    BooleanScript,
    /// Rust 历史兼容别名；新规则应使用 BooleanScript。
    IfScript,
    ForScript,
    /// Rust 历史兼容别名。
    WhileScript,
    /// Rust 历史兼容别名。
    BreakScript,
    /// Java v2.16 的降级节点。
    Fallback,
}

impl NodeTypeEnum {
    /// getCode()：规则文件中的 type 字符串
    pub fn get_code(&self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Switch => "switch",
            Self::Boolean => "boolean",
            Self::If => "if",
            Self::For => "for",
            Self::While => "while",
            Self::Break => "break",
            Self::Iterator => "iterator",
            Self::Script => "script",
            Self::SwitchScript => "switch_script",
            Self::BooleanScript => "boolean_script",
            Self::IfScript => "if_script",
            Self::ForScript => "for_script",
            Self::WhileScript => "while_script",
            Self::BreakScript => "break_script",
            Self::Fallback => "fallback",
        }
    }
    /// getName()：中文显示名（对齐 Java name 字段）
    pub fn get_name(&self) -> &'static str {
        match self {
            Self::Common => "普通",
            Self::Switch => "选择",
            Self::Boolean => "布尔",
            Self::If => "条件",
            Self::For => "循环次数",
            Self::While => "循环条件",
            Self::Break => "循环跳出",
            Self::Iterator => "循环迭代",
            Self::Script => "脚本",
            Self::SwitchScript => "选择脚本",
            Self::BooleanScript => "布尔脚本",
            Self::IfScript => "条件脚本",
            Self::ForScript => "循环次数脚本",
            Self::WhileScript => "循环条件脚本",
            Self::BreakScript => "循环跳出脚本",
            Self::Fallback => "降级",
        }
    }
    /// isScript()：是否脚本节点
    pub fn is_script(&self) -> bool {
        matches!(
            self,
            Self::Script
                | Self::SwitchScript
                | Self::BooleanScript
                | Self::IfScript
                | Self::ForScript
                | Self::WhileScript
                | Self::BreakScript
        )
    }
    /// getEnumByCode(code)
    pub fn get_enum_by_code(code: &str) -> Option<Self> {
        [
            Self::Common,
            Self::Switch,
            Self::Boolean,
            Self::If,
            Self::For,
            Self::While,
            Self::Break,
            Self::Iterator,
            Self::Script,
            Self::SwitchScript,
            Self::BooleanScript,
            Self::IfScript,
            Self::ForScript,
            Self::WhileScript,
            Self::BreakScript,
            Self::Fallback,
        ]
        .into_iter()
        .find(|e| e.get_code() == code)
    }
}
