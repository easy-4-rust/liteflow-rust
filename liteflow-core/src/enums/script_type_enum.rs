//! 对应 com.yomahub.liteflow.enums.ScriptTypeEnum：
//! 脚本引擎类型（engineName / displayName）。
//! Rust 端保留 Java 的全部公开枚举项；具体 JVM 引擎由独立受控执行器做 Rust 化映射。

/// 脚本引擎类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptTypeEnum {
    Custom,
    Groovy,
    QlExpress,
    Js,
    Python,
    Lua,
    Aviator,
    Java,
    Kotlin,
    /// Rust 端扩展：rhai 引擎（对齐 qlexpress 的嵌入式表达式生态位）
    Rhai,
}

impl ScriptTypeEnum {
    /// getEngineName()：JSR223 引擎名（Rust 端为等价引擎标识）
    pub fn get_engine_name(&self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Groovy => "groovy",
            Self::QlExpress => "qlexpress",
            Self::Js => "javascript",
            Self::Python => "python",
            Self::Lua => "luaj",
            Self::Aviator => "AviatorScript",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Rhai => "rhai",
        }
    }
    /// getDisplayName()：规则文件 language 属性值
    pub fn get_display_name(&self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Groovy => "groovy",
            Self::QlExpress => "qlexpress",
            Self::Js => "js",
            Self::Python => "python",
            Self::Lua => "lua",
            Self::Aviator => "aviator",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Rhai => "rhai",
        }
    }
    /// getEnumByDisplayName(displayName)
    pub fn get_enum_by_display_name(display_name: &str) -> Option<Self> {
        [
            Self::Custom,
            Self::Groovy,
            Self::QlExpress,
            Self::Js,
            Self::Python,
            Self::Lua,
            Self::Aviator,
            Self::Java,
            Self::Kotlin,
            Self::Rhai,
        ]
        .into_iter()
        .find(|e| e.get_display_name() == display_name)
    }

    /// 校验规则文件中的脚本类型是否合法。
    ///
    /// 对应 Java: `ScriptTypeEnum#checkScriptType`。
    #[must_use]
    pub fn check_script_type(script_type: &str) -> bool {
        Self::get_enum_by_display_name(script_type).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::ScriptTypeEnum;

    #[test]
    fn keeps_java_engine_and_display_names() {
        assert_eq!(ScriptTypeEnum::Aviator.get_engine_name(), "AviatorScript");
        assert_eq!(ScriptTypeEnum::Kotlin.get_display_name(), "kotlin");
        assert_eq!(
            ScriptTypeEnum::get_enum_by_display_name("custom"),
            Some(ScriptTypeEnum::Custom)
        );
        assert!(ScriptTypeEnum::check_script_type("java"));
        assert!(!ScriptTypeEnum::check_script_type("unknown"));
    }
}
