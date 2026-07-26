//! 对应 Java: com.yomahub.liteflow.script.annotation.ScriptBean

/// 脚本 Bean 的暴露规则。
///
/// `include_method_names` 非空时只允许其中的方法，随后再应用排除列表；
/// 该优先级与 Java `ScriptBeanProxy` 保持一致。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptBean {
    name: String,
    include_method_names: Vec<String>,
    exclude_method_names: Vec<String>,
}

impl ScriptBean {
    /// 创建脚本 Bean 元数据。
    ///
    /// 参数 `name` 是脚本侧名称；空字符串表示使用对象类型名。
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// 设置允许暴露的方法名。
    #[must_use]
    pub fn include_method_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.include_method_names = names.into_iter().map(Into::into).collect();
        self
    }

    /// 设置禁止暴露的方法名。
    #[must_use]
    pub fn exclude_method_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.exclude_method_names = names.into_iter().map(Into::into).collect();
        self
    }

    /// 返回脚本侧 Bean 名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回允许列表。
    #[must_use]
    pub fn includes(&self) -> &[String] {
        &self.include_method_names
    }

    /// 返回排除列表。
    #[must_use]
    pub fn excludes(&self) -> &[String] {
        &self.exclude_method_names
    }
}
