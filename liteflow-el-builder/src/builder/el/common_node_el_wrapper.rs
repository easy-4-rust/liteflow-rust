use super::el_wrapper::{
    ELBuilderResult, ELWrapper, ELWrapperData, RenderMode, WrapperKind, escape_el_string, tabs,
};

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
        output.push_str(&render_node_id(&self.node_id));
        self.properties
            .append_properties(&mut output, param_context, mode);
        Ok(output)
    }
}

/// 将节点 ID 渲染为 QLExpress 可无歧义解析的表达式。
///
/// Java EL Builder 最终交给 QLExpress 解析。普通标识符保持紧凑输出；Java
/// 关键字或不能作为 QL 标识符的节点引用必须改用 `node("id")`，否则会被
/// QLExpress 当成 `continue`、`break` 等语句或拆成多个 token；节点 ID 自身的
/// 合法性仍由 Core 的 Java 对等校验负责。
fn render_node_id(node_id: &str) -> String {
    if is_ql_identifier(node_id) && !is_ql_keyword(node_id) {
        return node_id.to_string();
    }
    format!("node(\"{}\")", escape_el_string(node_id))
}

fn is_ql_identifier(node_id: &str) -> bool {
    let mut characters = node_id.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first == '$' || first.is_alphabetic())
        && characters
            .all(|character| character == '_' || character == '$' || character.is_alphanumeric())
}

fn is_ql_keyword(node_id: &str) -> bool {
    matches!(
        node_id,
        "for"
            | "if"
            | "else"
            | "while"
            | "break"
            | "continue"
            | "return"
            | "function"
            | "macro"
            | "import"
            | "static"
            | "new"
            | "switch"
            | "case"
            | "default"
            | "byte"
            | "short"
            | "int"
            | "long"
            | "float"
            | "double"
            | "char"
            | "boolean"
            | "null"
            | "true"
            | "false"
            | "extends"
            | "super"
            | "try"
            | "catch"
            | "finally"
            | "throw"
            | "then"
            | "class"
            | "this"
    )
}
