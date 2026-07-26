use super::el_bus::ELBus;
use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, IntoELWrapper, RenderMode,
    WrapperKind, render_call,
};

/// 与或非表达式中的 OR 表达式。
///
/// 参数数量任意，所有参数必须能返回布尔值；可继续调用 `or` 追加表达式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.OrELWrapper`。
pub struct OrELWrapper {
    children: Vec<BoxedELWrapper>,
    properties: ELWrapperData,
}

impl OrELWrapper {
    pub(crate) fn new(children: Vec<BoxedELWrapper>) -> Self {
        Self {
            children,
            properties: ELWrapperData::default(),
        }
    }

    /// 追加能返回布尔值的表达式。对应 Java: `OrELWrapper#or`。
    pub fn or<I, T>(mut self, items: I) -> ELBuilderResult<Self>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        self.children.extend(ELBus::convert_boolean(items)?);
        Ok(self)
    }

    /// 设置组件标记内容。
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.properties.set_tag(tag);
        self
    }

    /// 设置组件实例 id。
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.properties.set_id(id);
        self
    }
}

impl ELWrapper for OrELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::BooleanOperator
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        let mut output = render_call("OR", &self.children, depth, param_context, mode)?;
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
