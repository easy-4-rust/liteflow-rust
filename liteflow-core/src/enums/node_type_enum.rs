//! 对应 com.yomahub.liteflow.enums.NodeTypeEnum：
//! 节点类型枚举（普通/选择/条件/循环/跳出/迭代 + 脚本系）。
//! Java 每个枚举携带 code、name、isScript 及映射的组件类；
//! Rust 端组件类映射由返回值类型约定表达（见 docs/语义迁移对照表.md 第三章）。

/// 节点类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTypeEnum {
    Common,
    Switch,
    If,
    For,
    While,
    Break,
    Iterator,
    Script,
    SwitchScript,
    IfScript,
    ForScript,
    WhileScript,
    BreakScript,
}

impl NodeTypeEnum {
    /// getCode()：规则文件中的 type 字符串
    pub fn get_code(&self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Switch => "switch",
            Self::If => "if",
            Self::For => "for",
            Self::While => "while",
            Self::Break => "break",
            Self::Iterator => "iterator",
            Self::Script => "script",
            Self::SwitchScript => "switch_script",
            Self::IfScript => "if_script",
            Self::ForScript => "for_script",
            Self::WhileScript => "while_script",
            Self::BreakScript => "break_script",
        }
    }
    /// getName()：中文显示名（对齐 Java name 字段）
    pub fn get_name(&self) -> &'static str {
        match self {
            Self::Common => "普通",
            Self::Switch => "选择",
            Self::If => "条件",
            Self::For => "循环次数",
            Self::While => "循环条件",
            Self::Break => "循环跳出",
            Self::Iterator => "循环迭代",
            Self::Script => "脚本",
            Self::SwitchScript => "选择脚本",
            Self::IfScript => "条件脚本",
            Self::ForScript => "循环次数脚本",
            Self::WhileScript => "循环条件脚本",
            Self::BreakScript => "循环跳出脚本",
        }
    }
    /// isScript()：是否脚本节点
    pub fn is_script(&self) -> bool {
        matches!(
            self,
            Self::Script | Self::SwitchScript | Self::IfScript | Self::ForScript | Self::WhileScript | Self::BreakScript
        )
    }
    /// getEnumByCode(code)
    pub fn get_enum_by_code(code: &str) -> Option<Self> {
        [
            Self::Common, Self::Switch, Self::If, Self::For, Self::While, Self::Break,
            Self::Iterator, Self::Script, Self::SwitchScript, Self::IfScript,
            Self::ForScript, Self::WhileScript, Self::BreakScript,
        ]
        .into_iter()
        .find(|e| e.get_code() == code)
    }
}
