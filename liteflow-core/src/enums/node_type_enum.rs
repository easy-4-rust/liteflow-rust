//! 对应 com.yomahub.liteflow.enums.NodeTypeEnum：
//! 节点类型枚举（普通/选择/条件/循环/跳出/迭代 + 脚本系）。
//! Java 每个枚举携带 code、name、isScript 及映射的组件类；
//! Rust 端组件类映射由返回值类型约定表达（见 docs/语义迁移对照表.md 第三章）。

use crate::core::NodeComponent;

/// 节点类型枚举。
///
/// 对应 Java: `com.yomahub.liteflow.enums.NodeTypeEnum`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// 返回规则文件中的类型代码。对应 Java: `NodeTypeEnum#getCode`。
    #[must_use]
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

    /// 根据类型代码更新节点类型。
    ///
    /// Java 枚举允许直接改写 `code` 字段；Rust 判别式枚举把代码与类型绑定，
    /// 因而在代码有效时把当前值切换为对应类型，无效代码返回 `false` 且保持原值。
    /// 参数 `code` 对应 Java 同名参数。对应 Java: `NodeTypeEnum#setCode`。
    pub fn set_code(&mut self, code: &str) -> bool {
        let Some(node_type) = Self::get_enum_by_code(code) else {
            return false;
        };
        *self = node_type;
        true
    }

    /// 返回中文显示名。对应 Java: `NodeTypeEnum#getName`。
    #[must_use]
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

    /// 根据中文显示名更新节点类型。
    ///
    /// 参数 `name` 对应 Java 同名参数；未知名称返回 `false` 且保持原值。
    /// 对应 Java: `NodeTypeEnum#setName`。
    pub fn set_name(&mut self, name: &str) -> bool {
        let Some(node_type) = Self::values()
            .into_iter()
            .find(|node_type| node_type.get_name() == name)
        else {
            return false;
        };
        *self = node_type;
        true
    }

    /// 返回是否为脚本节点。对应 Java: `NodeTypeEnum#isScript`。
    #[must_use]
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

    /// 切换当前类型的脚本属性。
    ///
    /// Java 可以独立改写布尔字段；Rust 将脚本属性与组件类别绑定，因此在普通、
    /// 选择、布尔、循环次数之间切换其脚本/非脚本对等类型。迭代和降级类型没有
    /// Java 脚本对等项，设置为 `true` 时返回 `false`。
    /// 参数 `script` 对应 Java 同名参数。对应 Java: `NodeTypeEnum#setScript`。
    pub fn set_script(&mut self, script: bool) -> bool {
        let node_type = match (*self, script) {
            (Self::Common | Self::Script, false) => Self::Common,
            (Self::Common | Self::Script, true) => Self::Script,
            (Self::Switch | Self::SwitchScript, false) => Self::Switch,
            (Self::Switch | Self::SwitchScript, true) => Self::SwitchScript,
            (
                Self::Boolean
                | Self::If
                | Self::While
                | Self::Break
                | Self::BooleanScript
                | Self::IfScript
                | Self::WhileScript
                | Self::BreakScript,
                false,
            ) => Self::Boolean,
            (
                Self::Boolean
                | Self::If
                | Self::While
                | Self::Break
                | Self::BooleanScript
                | Self::IfScript
                | Self::WhileScript
                | Self::BreakScript,
                true,
            ) => Self::BooleanScript,
            (Self::For | Self::ForScript, false) => Self::For,
            (Self::For | Self::ForScript, true) => Self::ForScript,
            (Self::Iterator | Self::Fallback, false) => *self,
            (Self::Iterator | Self::Fallback, true) => return false,
        };
        *self = node_type;
        true
    }

    /// 返回 Java 映射组件类的稳定名称。
    ///
    /// Rust 不使用 `Class<? extends NodeComponent>` 反射，故以组件类别名称表达
    /// 相同映射；降级类型与 Java 一致返回 `None`。
    /// 对应 Java: `NodeTypeEnum#getMappingClazz`。
    #[must_use]
    pub fn get_mapping_clazz(&self) -> Option<&'static str> {
        match self {
            Self::Common => Some("NodeComponent"),
            Self::Script => Some("ScriptCommonComponent"),
            Self::Switch => Some("NodeSwitchComponent"),
            Self::SwitchScript => Some("ScriptSwitchComponent"),
            Self::Boolean | Self::If | Self::While | Self::Break => Some("NodeBooleanComponent"),
            Self::BooleanScript | Self::IfScript | Self::WhileScript | Self::BreakScript => {
                Some("ScriptBooleanComponent")
            }
            Self::For => Some("NodeForComponent"),
            Self::ForScript => Some("ScriptForComponent"),
            Self::Iterator => Some("NodeIteratorComponent"),
            Self::Fallback => None,
        }
    }

    /// 根据组件类别名称更新映射的节点类型。
    ///
    /// 参数 `mapping_clazz` 对应 Java 同名参数；Rust 以稳定类名替代 JVM
    /// `Class`。未知名称返回 `false`。
    /// 对应 Java: `NodeTypeEnum#setMappingClazz`。
    pub fn set_mapping_clazz(&mut self, mapping_clazz: Option<&str>) -> bool {
        let Some(mapping_clazz) = mapping_clazz else {
            *self = Self::Fallback;
            return true;
        };
        let Some(node_type) = Self::guess_type_by_super_clazz(mapping_clazz) else {
            return false;
        };
        *self = node_type;
        true
    }

    /// 按规则类型代码查找枚举值。
    ///
    /// 参数 `code` 为规则文件中的 `type`；未命中返回 `None`。
    /// 对应 Java: `NodeTypeEnum#getEnumByCode`。
    #[must_use]
    pub fn get_enum_by_code(code: &str) -> Option<Self> {
        Self::values()
            .into_iter()
            .find(|node_type| node_type.get_code() == code)
    }

    /// 根据组件父类名称推断节点类型。
    ///
    /// Rust 没有 JVM 父类反射，调用方传入注册元数据中的组件类名或限定名；
    /// 本方法取最后一级类型名完成映射。对应 Java:
    /// `NodeTypeEnum#guessTypeBySuperClazz`。
    #[must_use]
    pub fn guess_type_by_super_clazz(mapping_clazz: &str) -> Option<Self> {
        let simple_name = mapping_clazz
            .rsplit(['.', ':'])
            .next()
            .unwrap_or(mapping_clazz);
        match simple_name {
            "NodeComponent" => Some(Self::Common),
            "ScriptCommonComponent" => Some(Self::Script),
            "NodeSwitchComponent" => Some(Self::Switch),
            "ScriptSwitchComponent" => Some(Self::SwitchScript),
            "NodeBooleanComponent" => Some(Self::Boolean),
            "ScriptBooleanComponent" => Some(Self::BooleanScript),
            "NodeForComponent" => Some(Self::For),
            "ScriptForComponent" => Some(Self::ForScript),
            "NodeIteratorComponent" => Some(Self::Iterator),
            _ => None,
        }
    }

    /// 根据组件公开的节点类型元数据进行推断。
    ///
    /// 参数 `component` 对应 Java `clazz` 的 Rust 运行时对象替代；显式注册类型
    /// 存在时直接返回，否则无法可靠推断并返回 `None`。
    /// 对应 Java: `NodeTypeEnum#guessType`。
    #[must_use]
    pub fn guess_type(component: &dyn NodeComponent) -> Option<Self> {
        component.node_type()
    }

    fn values() -> [Self; 16] {
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
    }
}
