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
    /// 保留的 AST 兼容字段；Java Node 分支忽略 bind 的 override 参数。
    pub bind_override: bool,
    /// 首个把 Chain 转成 Condition 的操作是否为 tag。
    ///
    /// Java `Chain.tag` 会创建 ThenCondition，而 `Chain.bind` 会创建
    /// ChainBindWrapperCondition；解析阶段尚不能区分 Node 与 Chain，因此暂存
    /// 首个包装操作，交给 EL Builder 在解析注册表后还原真实类型。
    pub(crate) chain_tag_wrapper: bool,
    /// tag/bind 把 Chain 包装成 Condition 后设置的 Condition ID。
    pub(crate) condition_id: Option<String>,
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
            chain_tag_wrapper: false,
            condition_id: None,
        }
    }

    /// 返回展示名；与 Java `Node#getDisplayName` 一致，优先使用实例别名。
    #[must_use]
    pub fn display(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.id)
    }
}
