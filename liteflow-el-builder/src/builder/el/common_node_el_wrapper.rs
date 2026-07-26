use super::el_wrapper::{ELBuilderResult, ELWrapper, ELWrapperData, RenderMode, WrapperKind, tabs};

/// 普通节点表示。
///
/// 普通节点既可以作为布尔判断节点，也可以作为普通执行节点。
/// 对应 Java: `com.yomahub.liteflow.builder.el.CommonNodeELWrapper`。
#[derive(Debug, Clone)]
pub struct CommonNodeELWrapper {
    node_id: String,
    pub(crate) properties: ELWrapperData,
}

impl CommonNodeELWrapper {
    /// 创建普通节点表达式。
    ///
    /// # 参数
    /// - `node_id`: LiteFlow 节点或子链 id。
    ///
    /// # 返回
    /// 普通节点包装器。
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            properties: ELWrapperData::default(),
        }
    }

    /// 返回节点 id。
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    super::el_wrapper::impl_common_fluent!(CommonNodeELWrapper);
}

impl ELWrapper for CommonNodeELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::CommonNode
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        let mut output = tabs(depth);
        output.push_str(&self.node_id);
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
