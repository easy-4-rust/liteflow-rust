use super::el_bus::ELBus;
use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, IntoELWrapper, RenderMode,
    WrapperKind, newline, tabs,
};

/// 捕获异常表达式：`CATCH(a).DO(b)`。
///
/// CATCH 和 DO 均只允许一个非布尔运算表达式。
/// 对应 Java: `com.yomahub.liteflow.builder.el.CatchELWrapper`。
pub struct CatchELWrapper {
    body: BoxedELWrapper,
    fallback: Option<BoxedELWrapper>,
    pub(crate) properties: ELWrapperData,
}

impl CatchELWrapper {
    pub(crate) fn new(body: BoxedELWrapper) -> Self {
        Self {
            body,
            fallback: None,
            properties: ELWrapperData::default(),
        }
    }

    /// 设置发生异常后执行的表达式。对应 Java: `CatchELWrapper#doOpt`。
    pub fn do_opt<T: IntoELWrapper>(mut self, item: T) -> ELBuilderResult<Self> {
        self.fallback = Some(ELBus::convert_one_non_boolean(item)?);
        Ok(self)
    }

    super::el_wrapper::impl_common_fluent!(CatchELWrapper);
}

impl ELWrapper for CatchELWrapper {
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
        let mut output = tabs(depth);
        output.push_str("CATCH(");
        newline(&mut output, depth);
        output.push_str(&self.body.render_el(child_depth, param_context, mode)?);
        newline(&mut output, depth);
        output.push_str(&tabs(depth));
        output.push(')');
        if let Some(fallback) = &self.fallback {
            output.push_str(".DO(");
            newline(&mut output, depth);
            output.push_str(&fallback.render_el(child_depth, param_context, mode)?);
            newline(&mut output, depth);
            output.push_str(&tabs(depth));
            output.push(')');
        }
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
