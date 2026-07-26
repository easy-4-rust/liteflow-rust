use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, RenderMode, WrapperKind, render_call,
};

/// 与或非表达式中的 NOT 表达式。
///
/// 只允许一个能返回布尔值的参数。
/// 对应 Java: `com.yomahub.liteflow.builder.el.NotELWrapper`。
pub struct NotELWrapper {
    child: BoxedELWrapper,
    properties: ELWrapperData,
}

impl NotELWrapper {
    pub(crate) fn new(child: BoxedELWrapper) -> Self {
        Self {
            child,
            properties: ELWrapperData::default(),
        }
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

impl ELWrapper for NotELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::BooleanOperator
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        let mut output = render_call(
            "NOT",
            std::slice::from_ref(&self.child),
            depth,
            param_context,
            mode,
        )?;
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
