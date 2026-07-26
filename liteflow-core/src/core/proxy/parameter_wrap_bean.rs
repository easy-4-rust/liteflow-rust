//! Java `ParameterWrapBean` 的 Rust 映射。

/// 保存声明式方法参数的类型、事实 Bean 名称及参数位置。
///
/// Java 通过 `Class<?>` 与 `@LiteflowFact` 做运行期反射注入；Rust 由
/// `liteflow-derive` 生成强类型 downcast，本对象保留类型名和事实名称，用于代理
/// 调用前的结构校验及错误诊断。
///
/// 对应 Java: `com.yomahub.liteflow.core.proxy.ParameterWrapBean`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterWrapBean {
    parameter_type: String,
    fact: Option<String>,
    index: usize,
}

impl ParameterWrapBean {
    /// 创建参数包装对象。
    ///
    /// 对应 Java: `ParameterWrapBean#ParameterWrapBean`。
    #[must_use]
    pub fn new(
        parameter_type: impl Into<String>,
        fact: Option<impl Into<String>>,
        index: usize,
    ) -> Self {
        Self {
            parameter_type: parameter_type.into(),
            fact: fact.map(Into::into),
            index,
        }
    }

    /// 返回 Rust 参数类型名称。对应 Java: `ParameterWrapBean#getParameterType`。
    #[must_use]
    pub fn parameter_type(&self) -> &str {
        &self.parameter_type
    }

    /// 修改 Rust 参数类型名称。对应 Java: `ParameterWrapBean#setParameterType`。
    pub fn set_parameter_type(&mut self, parameter_type: impl Into<String>) {
        self.parameter_type = parameter_type.into();
    }

    /// 返回 `#[liteflow_fact]` 指定的 Bean 名称。
    ///
    /// 对应 Java: `ParameterWrapBean#getFact`。
    #[must_use]
    pub fn fact(&self) -> Option<&str> {
        self.fact.as_deref()
    }

    /// 修改事实 Bean 名称。对应 Java: `ParameterWrapBean#setFact`。
    pub fn set_fact(&mut self, fact: Option<impl Into<String>>) {
        self.fact = fact.map(Into::into);
    }

    /// 返回参数在原方法中的位置。对应 Java: `ParameterWrapBean#getIndex`。
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// 修改参数位置。对应 Java: `ParameterWrapBean#setIndex`。
    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}
