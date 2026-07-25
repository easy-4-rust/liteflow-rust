//! 对应 com.yomahub.liteflow.enums.ScriptTypeEnum：
//! 脚本引擎类型（engineName / displayName）。
//! Rust 端生态映射：qlexpress 生态位 → rhai；luaj → mlua；
//! javascript → boa/quickjs（路线图）；groovy/python 为 JVM 生态特有。

/// 脚本引擎类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptTypeEnum {
    Groovy,
    QlExpress,
    Js,
    Python,
    Lua,
    /// Rust 端扩展：rhai 引擎（对齐 qlexpress 的嵌入式表达式生态位）
    Rhai,
}

impl ScriptTypeEnum {
    /// getEngineName()：JSR223 引擎名（Rust 端为等价引擎标识）
    pub fn get_engine_name(&self) -> &'static str {
        match self {
            Self::Groovy => "groovy",
            Self::QlExpress => "qlexpress",
            Self::Js => "javascript",
            Self::Python => "python",
            Self::Lua => "luaj",
            Self::Rhai => "rhai",
        }
    }
    /// getDisplayName()：规则文件 language 属性值
    pub fn get_display_name(&self) -> &'static str {
        match self {
            Self::Groovy => "groovy",
            Self::QlExpress => "qlexpress",
            Self::Js => "js",
            Self::Python => "python",
            Self::Lua => "lua",
            Self::Rhai => "rhai",
        }
    }
    /// getEnumByDisplayName(displayName)
    pub fn get_enum_by_display_name(display_name: &str) -> Option<Self> {
        [Self::Groovy, Self::QlExpress, Self::Js, Self::Python, Self::Lua, Self::Rhai]
            .into_iter()
            .find(|e| e.get_display_name() == display_name)
    }
}
