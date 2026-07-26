use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, RenderMode, WrapperKind, render_call,
};

/// 前置表达式。
///
/// 只能在 THEN/SER 组件中调用；参数数量不限，类型不能是 AND/OR/NOT 表达式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.PreELWrapper`。
pub struct PreELWrapper {
    children: Vec<BoxedELWrapper>,
    pub(crate) properties: ELWrapperData,
}

impl PreELWrapper {
    pub(crate) fn new(children: Vec<BoxedELWrapper>) -> Self {
        Self {
            children,
            properties: ELWrapperData::default(),
        }
    }

    super::el_wrapper::impl_common_fluent!(PreELWrapper);
}

impl ELWrapper for PreELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::NonBoolean
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        let mut output = render_call("PRE", &self.children, depth, param_context, mode)?;
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
