use super::el_bus::ELBus;
use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, IntoELWrapper, RenderMode,
    WrapperKind, newline, tabs,
};

/// IF/ELIF/ELSE 条件表达式。
///
/// 判断位置允许普通节点和 AND/OR/NOT；执行分支不允许 AND/OR/NOT。
/// 内部以 Rust 向量保存 ELIF 链，输出语义等价于 Java 版的嵌套 IfELWrapper 树。
/// 对应 Java: `com.yomahub.liteflow.builder.el.IfELWrapper`。
pub struct IfELWrapper {
    condition: BoxedELWrapper,
    true_branch: BoxedELWrapper,
    elif_branches: Vec<(BoxedELWrapper, BoxedELWrapper)>,
    false_branch: Option<BoxedELWrapper>,
    pub(crate) properties: ELWrapperData,
}

impl IfELWrapper {
    pub(crate) fn new(
        condition: BoxedELWrapper,
        true_branch: BoxedELWrapper,
        false_branch: Option<BoxedELWrapper>,
    ) -> Self {
        Self {
            condition,
            true_branch,
            elif_branches: Vec::new(),
            false_branch,
            properties: ELWrapperData::default(),
        }
    }

    /// 设置 ELSE 分支。对应 Java: `IfELWrapper#elseOpt`。
    pub fn else_opt<T: IntoELWrapper>(mut self, item: T) -> ELBuilderResult<Self> {
        self.false_branch = Some(ELBus::convert_one_non_boolean(item)?);
        Ok(self)
    }

    /// 追加 ELIF 判断和执行分支。对应 Java: `IfELWrapper#elIfOpt`。
    pub fn el_if_opt<C, T>(mut self, condition: C, true_branch: T) -> ELBuilderResult<Self>
    where
        C: IntoELWrapper,
        T: IntoELWrapper,
    {
        self.elif_branches.push((
            ELBus::convert_one_boolean(condition)?,
            ELBus::convert_one_non_boolean(true_branch)?,
        ));
        Ok(self)
    }

    super::el_wrapper::impl_common_fluent!(IfELWrapper);
}

impl ELWrapper for IfELWrapper {
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
        output.push_str("IF(");
        newline(&mut output, depth);
        output.push_str(&self.condition.render_el(child_depth, param_context, mode)?);
        output.push(',');
        newline(&mut output, depth);
        output.push_str(
            &self
                .true_branch
                .render_el(child_depth, param_context, mode)?,
        );

        if self.elif_branches.is_empty() {
            if let Some(false_branch) = &self.false_branch {
                output.push(',');
                newline(&mut output, depth);
                output.push_str(&false_branch.render_el(child_depth, param_context, mode)?);
            }
            newline(&mut output, depth);
            output.push_str(&tabs(depth));
            output.push(')');
        } else {
            newline(&mut output, depth);
            output.push_str(&tabs(depth));
            output.push(')');
            for (condition, branch) in &self.elif_branches {
                output.push_str(".ELIF(");
                newline(&mut output, depth);
                output.push_str(&condition.render_el(child_depth, param_context, mode)?);
                output.push(',');
                newline(&mut output, depth);
                output.push_str(&branch.render_el(child_depth, param_context, mode)?);
                newline(&mut output, depth);
                output.push_str(&tabs(depth));
                output.push(')');
            }
            if let Some(false_branch) = &self.false_branch {
                output.push_str(".ELSE(");
                newline(&mut output, depth);
                output.push_str(&false_branch.render_el(child_depth, param_context, mode)?);
                newline(&mut output, depth);
                output.push_str(&tabs(depth));
                output.push(')');
            }
        }
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
