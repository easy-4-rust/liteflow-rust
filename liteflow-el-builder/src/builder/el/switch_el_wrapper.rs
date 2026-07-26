use super::el_bus::ELBus;
use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, IntoELWrapper, RenderMode,
    WrapperKind, newline, tabs,
};

/// 选择组件：`SWITCH(a).TO(b,c).DEFAULT(x)`。
///
/// SWITCH 和 DEFAULT 只允许单个非布尔运算表达式，TO 参数数量任意。
/// 对应 Java: `com.yomahub.liteflow.builder.el.SwitchELWrapper`。
pub struct SwitchELWrapper {
    selector: BoxedELWrapper,
    targets: Vec<BoxedELWrapper>,
    default_target: Option<BoxedELWrapper>,
    pub(crate) properties: ELWrapperData,
}

impl SwitchELWrapper {
    pub(crate) fn new(selector: BoxedELWrapper) -> Self {
        Self {
            selector,
            targets: Vec::new(),
            default_target: None,
            properties: ELWrapperData::default(),
        }
    }

    /// 追加候选目标表达式。对应 Java: `SwitchELWrapper#to`。
    pub fn to<I, T>(mut self, items: I) -> ELBuilderResult<Self>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        self.targets.extend(ELBus::convert_non_boolean(items)?);
        Ok(self)
    }

    /// 设置默认目标表达式。对应 Java: `SwitchELWrapper#defaultOpt`。
    pub fn default_opt<T: IntoELWrapper>(mut self, item: T) -> ELBuilderResult<Self> {
        self.default_target = Some(ELBus::convert_one_non_boolean(item)?);
        Ok(self)
    }

    super::el_wrapper::impl_common_fluent!(SwitchELWrapper);
}

impl ELWrapper for SwitchELWrapper {
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
        let mut output = format!(
            "{}SWITCH({})",
            tabs(depth),
            self.selector.render_el(None, param_context, mode)?
        );
        if !self.targets.is_empty() {
            output.push_str(".TO(");
            newline(&mut output, depth);
            for (index, target) in self.targets.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                    newline(&mut output, depth);
                }
                output.push_str(&target.render_el(child_depth, param_context, mode)?);
            }
            newline(&mut output, depth);
            output.push_str(&tabs(depth));
            output.push(')');
        }
        if let Some(default_target) = &self.default_target {
            output.push_str(".DEFAULT(");
            newline(&mut output, depth);
            output.push_str(&default_target.render_el(child_depth, param_context, mode)?);
            newline(&mut output, depth);
            output.push_str(&tabs(depth));
            output.push(')');
        }
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
