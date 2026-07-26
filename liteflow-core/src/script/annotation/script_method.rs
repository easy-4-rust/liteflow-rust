//! 对应 Java: com.yomahub.liteflow.script.annotation.ScriptMethod

/// 单个脚本方法的暴露名称。
///
/// 名称为空时沿用 Rust 方法名，对应 Java 注解 `value` 的默认语义。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptMethod {
    exposed_name: String,
}

impl ScriptMethod {
    /// 创建脚本方法元数据。
    #[must_use]
    pub fn new(exposed_name: impl Into<String>) -> Self {
        Self {
            exposed_name: exposed_name.into(),
        }
    }

    /// 用声明值或原方法名解析最终暴露名称。
    #[must_use]
    pub fn resolve_name<'a>(&'a self, method_name: &'a str) -> &'a str {
        if self.exposed_name.is_empty() {
            method_name
        } else {
            &self.exposed_name
        }
    }
}
