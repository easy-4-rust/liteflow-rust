use serde::Serialize;

use super::el_wrapper::{
    BoxedELWrapper, ELBuilderResult, ELWrapper, ELWrapperData, RenderMode, WrapperKind, render_call,
};
use super::vo::RetryELVo;

/// 后置表达式。
///
/// 只能在 THEN/SER 组件中调用；参数数量不限，类型不能是 AND/OR/NOT 表达式。
/// Java 版虽然保留 data/bind/retry 设置入口，但最终只输出 id/tag，本实现保持该语义。
/// 对应 Java: `com.yomahub.liteflow.builder.el.FinallyELWrapper`。
pub struct FinallyELWrapper {
    children: Vec<BoxedELWrapper>,
    properties: ELWrapperData,
}

impl FinallyELWrapper {
    pub(crate) fn new(children: Vec<BoxedELWrapper>) -> Self {
        Self {
            children,
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

    /// 设置 data；与 Java 相同，FINALLY 输出阶段不会写出该属性。
    pub fn data<T: Serialize>(
        mut self,
        data_name: impl Into<String>,
        value: &T,
    ) -> ELBuilderResult<Self> {
        self.properties.set_data(data_name, value)?;
        Ok(self)
    }

    /// 设置 JSON data；与 Java 相同，FINALLY 输出阶段不会写出该属性。
    pub fn data_json(mut self, data_name: impl Into<String>, json: impl Into<String>) -> Self {
        self.properties.set_data_json(data_name, json);
        self
    }

    /// 设置 bind；与 Java 相同，FINALLY 输出阶段不会写出该属性。
    pub fn bind(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.bind(key, value);
        self
    }

    /// 设置 retry；与 Java 相同，FINALLY 输出阶段不会写出该属性。
    pub fn retry(mut self, count: u32) -> Self {
        self.properties.set_retry(RetryELVo::new(count));
        self
    }
}

impl ELWrapper for FinallyELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::NonBoolean
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        let mut output = render_call("FINALLY", &self.children, depth, param_context, mode)?;
        self.properties.append_id_and_tag(&mut output);
        Ok(output)
    }
}
