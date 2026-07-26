use super::el_bus::ELBus;
use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, IntoELWrapper, RenderMode,
    WrapperKind, newline, tabs,
};
use super::{FinallyELWrapper, PreELWrapper};

/// THEN 串行组件。
///
/// 允许调用 PRE、FINALLY 任意次；普通参数数量任意且不能是 AND/OR/NOT。
/// 对应 Java: `com.yomahub.liteflow.builder.el.ThenELWrapper`。
pub struct ThenELWrapper {
    children: Vec<BoxedELWrapper>,
    pre_wrappers: Vec<PreELWrapper>,
    finally_wrappers: Vec<FinallyELWrapper>,
    pub(crate) properties: ELWrapperData,
}

impl ThenELWrapper {
    pub(crate) fn new(children: Vec<BoxedELWrapper>) -> Self {
        Self {
            children,
            pre_wrappers: Vec::new(),
            finally_wrappers: Vec::new(),
            properties: ELWrapperData::default(),
        }
    }

    /// 追加串行子表达式。对应 Java: `ThenELWrapper#then`。
    pub fn then<I, T>(mut self, items: I) -> ELBuilderResult<Self>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        self.children.extend(ELBus::convert_non_boolean(items)?);
        Ok(self)
    }

    /// 在当前串行组件下创建前置组件。对应 Java: `ThenELWrapper#pre`。
    pub fn pre<I, T>(mut self, items: I) -> ELBuilderResult<Self>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        self.pre_wrappers
            .push(PreELWrapper::new(ELBus::convert_non_boolean(items)?));
        Ok(self)
    }

    /// 在当前串行组件下创建后置组件。对应 Java: `ThenELWrapper#finallyOpt`。
    pub fn finally_opt<I, T>(mut self, items: I) -> ELBuilderResult<Self>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        self.finally_wrappers
            .push(FinallyELWrapper::new(ELBus::convert_non_boolean(items)?));
        Ok(self)
    }

    super::el_wrapper::impl_common_fluent!(ThenELWrapper);
}

impl ELWrapper for ThenELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::NonBoolean
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        let child_depth = depth.map(|value| value + 1);
        let mut rendered = Vec::new();
        for wrapper in &self.pre_wrappers {
            rendered.push(wrapper.render_el(child_depth, param_context, mode)?);
        }
        for wrapper in &self.children {
            rendered.push(wrapper.render_el(child_depth, param_context, mode)?);
        }
        for wrapper in &self.finally_wrappers {
            rendered.push(wrapper.render_el(child_depth, param_context, mode)?);
        }

        let mut output = tabs(depth);
        output.push_str("THEN(");
        newline(&mut output, depth);
        for (index, expression) in rendered.iter().enumerate() {
            if index > 0 {
                output.push(',');
                newline(&mut output, depth);
            }
            output.push_str(expression);
        }
        newline(&mut output, depth);
        output.push_str(&tabs(depth));
        output.push(')');
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
