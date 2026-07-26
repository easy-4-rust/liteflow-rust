use super::el_bus::ELBus;
use super::el_wrapper::{
    BoxedELWrapper, ELBuilderError, ELBuilderResult, ELWrapper, ELWrapperData, IntoELWrapper,
    RenderMode, WrapperKind, newline, tabs,
};
use super::{LoopFunction, LoopSource};

/// FOR、WHILE、ITERATOR 循环表达式的公共包装器。
///
/// 以内部循环函数区分三类循环，并支持 `parallel`、`DO`、`BREAK`。
/// 对应 Java: `com.yomahub.liteflow.builder.el.LoopELWrapper`。
pub struct LoopELWrapper {
    source: LoopSource,
    function: LoopFunction,
    parallel: bool,
    body: Option<BoxedELWrapper>,
    break_condition: Option<BoxedELWrapper>,
    pub(crate) properties: ELWrapperData,
}

impl LoopELWrapper {
    pub(crate) fn for_count(loop_number: u32) -> Self {
        Self::new(LoopSource::Number(loop_number), LoopFunction::For)
    }

    pub(crate) fn for_expression(source: BoxedELWrapper) -> Self {
        Self::new(LoopSource::Expression(source), LoopFunction::For)
    }

    pub(crate) fn while_expression(source: BoxedELWrapper) -> Self {
        Self::new(LoopSource::Expression(source), LoopFunction::While)
    }

    pub(crate) fn iterator_expression(source: BoxedELWrapper) -> Self {
        Self::new(LoopSource::Expression(source), LoopFunction::Iterator)
    }

    fn new(source: LoopSource, function: LoopFunction) -> Self {
        Self {
            source,
            function,
            parallel: false,
            body: None,
            break_condition: None,
            properties: ELWrapperData::default(),
        }
    }

    /// 设置是否并行执行循环体。对应 Java: `LoopELWrapper#parallel`。
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// 设置循环体。对应 Java: `LoopELWrapper#doOpt`。
    pub fn do_opt<T: IntoELWrapper>(mut self, item: T) -> ELBuilderResult<Self> {
        self.body = Some(ELBus::convert_one_non_boolean(item)?);
        Ok(self)
    }

    /// 设置退出循环的布尔表达式。对应 Java: `LoopELWrapper#breakOpt`。
    pub fn break_opt<T: IntoELWrapper>(mut self, item: T) -> ELBuilderResult<Self> {
        self.break_condition = Some(ELBus::convert_one_boolean(item)?);
        Ok(self)
    }

    super::el_wrapper::impl_common_fluent!(LoopELWrapper);
}

impl ELWrapper for LoopELWrapper {
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
        output.push_str(self.function.as_str());
        output.push('(');
        match &self.source {
            LoopSource::Number(number) => output.push_str(&number.to_string()),
            LoopSource::Expression(source) => {
                newline(&mut output, depth);
                output.push_str(&source.render_el(child_depth, param_context, mode)?);
                newline(&mut output, depth);
                output.push_str(&tabs(depth));
            }
        }
        output.push(')');
        if self.parallel {
            output.push_str(".parallel(true)");
        }
        if let Some(body) = &self.body {
            output.push_str(".DO(");
            newline(&mut output, depth);
            output.push_str(&body.render_el(child_depth, param_context, mode)?);
            newline(&mut output, depth);
            output.push_str(&tabs(depth));
            output.push(')');
        }
        if let Some(condition) = &self.break_condition {
            output.push_str(".BREAK(");
            newline(&mut output, depth);
            output.push_str(&condition.render_el(child_depth, param_context, mode)?);
            newline(&mut output, depth);
            output.push_str(&tabs(depth));
            output.push(')');
        }
        if self.body.is_none() && self.break_condition.is_some() {
            return Err(ELBuilderError::MissingExpression(
                "BREAK 之前必须设置 DO 循环体".to_string(),
            ));
        }
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
