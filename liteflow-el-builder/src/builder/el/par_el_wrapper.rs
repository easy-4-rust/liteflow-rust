use super::el_bus::ELBus;
use super::el_wrapper::{
    BoxedELWrapper, ELBuilderError, ELBuilderResult, ELWrapper, ELWrapperData, IntoELWrapper,
    RenderMode, WrapperKind, escape_el_string, render_call,
};

/// PAR 并行组件。
///
/// 参数数量任意且不能是 AND/OR/NOT；支持 any、ignoreError、threadPool 和 must，
/// 其中 any 与 must 互斥。
/// 对应 Java: `com.yomahub.liteflow.builder.el.ParELWrapper`。
pub struct ParELWrapper {
    children: Vec<BoxedELWrapper>,
    any: bool,
    ignore_error: bool,
    custom_thread_executor: Option<String>,
    must_execute_list: Vec<String>,
    pub(crate) properties: ELWrapperData,
}

impl ParELWrapper {
    pub(crate) fn new(children: Vec<BoxedELWrapper>) -> Self {
        Self {
            children,
            any: false,
            ignore_error: false,
            custom_thread_executor: None,
            must_execute_list: Vec::new(),
            properties: ELWrapperData::default(),
        }
    }

    /// 追加并行子表达式。对应 Java: `ParELWrapper#par`。
    pub fn par<I, T>(mut self, items: I) -> ELBuilderResult<Self>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        self.children.extend(ELBus::convert_non_boolean(items)?);
        Ok(self)
    }

    /// 设置任意一个分支成功即成功。
    pub fn any(mut self, any: bool) -> Self {
        self.any = any;
        self
    }

    /// 设置是否忽略分支错误。
    pub fn ignore_error(mut self, ignore_error: bool) -> Self {
        self.ignore_error = ignore_error;
        self
    }

    /// 设置自定义线程执行器名称。
    pub fn custom_thread_executor(mut self, executor: impl Into<String>) -> Self {
        self.custom_thread_executor = Some(executor.into());
        self
    }

    /// 追加必须执行成功的节点 id。
    pub fn must<I, S>(mut self, node_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.must_execute_list
            .extend(node_ids.into_iter().map(Into::into));
        self
    }

    super::el_wrapper::impl_common_fluent!(ParELWrapper);
}

impl ELWrapper for ParELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::NonBoolean
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        if self.any && !self.must_execute_list.is_empty() {
            return Err(ELBuilderError::ConflictingOptions(
                "PAR 的 any 与 must 不能同时定义".to_string(),
            ));
        }
        let mut output = render_call("PAR", &self.children, depth, param_context, mode)?;
        if self.any {
            output.push_str(".any(true)");
        }
        if self.ignore_error {
            output.push_str(".ignoreError(true)");
        }
        if let Some(executor) = &self.custom_thread_executor {
            output.push_str(&format!(".threadPool(\"{}\")", escape_el_string(executor)));
        }
        if !self.must_execute_list.is_empty() {
            output.push_str(".must(");
            for (index, node_id) in self.must_execute_list.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!("\"{}\"", escape_el_string(node_id)));
            }
            output.push(')');
        }
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
