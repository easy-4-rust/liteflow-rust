use super::el_wrapper::{
    ELBuilderResult, ELWrapper, ELWrapperData, RenderMode, WrapperKind, escape_el_string, tabs,
};

/// 显式 `node("id")` 单节点表达式。
///
/// 当节点 id 需要降级为字符串参数形式时使用。
/// 对应 Java: `com.yomahub.liteflow.builder.el.NodeELWrapper`。
#[derive(Debug, Clone)]
pub struct NodeELWrapper {
    node_id: String,
    pub(crate) properties: ELWrapperData,
}

impl NodeELWrapper {
    /// 创建显式单节点表达式。
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

    super::el_wrapper::impl_common_fluent!(NodeELWrapper);
}

impl ELWrapper for NodeELWrapper {
    fn wrapper_kind(&self) -> WrapperKind {
        WrapperKind::CommonNode
    }

    fn render_el(
        &self,
        depth: Option<usize>,
        param_context: &mut String,
        mode: RenderMode,
    ) -> ELBuilderResult<String> {
        let mut output = format!(
            "{}node(\"{}\")",
            tabs(depth),
            escape_el_string(&self.node_id)
        );
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}
