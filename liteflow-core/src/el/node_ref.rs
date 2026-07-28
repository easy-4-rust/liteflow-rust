//! EL 节点引用。

/// 节点引用，对应 Java 的 Node 元素与 id/tag/data/bind 修饰。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct NodeRef {
    /// 节点 id。
    pub id: String,
    /// `.id("xxx")` 产生的实例别名。
    pub alias: Option<String>,
    /// 节点标签。
    pub tag: Option<String>,
    /// 节点元数据。
    pub data: Option<String>,
    /// 节点绑定数据。
    pub bind: Vec<(String, String)>,
    /// `.bind(k, v, override)` 的覆盖标记。
    pub bind_override: bool,
}

impl NodeRef {
    /// 根据节点 id 创建引用。
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            alias: None,
            tag: None,
            data: None,
            bind: Vec::new(),
            bind_override: false,
        }
    }

    /// 返回展示名；与 Java `Node#getDisplayName` 一致，优先使用实例别名。
    #[must_use]
    pub fn display(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.id)
    }
}
