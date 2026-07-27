/// 冒号形式脚本节点键的轻量转换对象。
///
/// 字段顺序依次为节点 id、节点类型、名称、脚本语言和启用状态；脚本正文由
/// 规则插件取得值后再写入。该对象对应 Java 静态内部类，因此与主对象保留在
/// 同一文件。
///
/// 对应 Java: `com.yomahub.liteflow.parser.helper.NodeConvertHelper.NodeSimpleVO`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSimpleVO {
    node_id: String,
    node_type: String,
    name: String,
    language: Option<String>,
    enable: bool,
    script: Option<String>,
}

impl NodeSimpleVO {
    fn new(node_id: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            node_type: node_type.into(),
            name: String::new(),
            language: None,
            enable: true,
            script: None,
        }
    }

    /// 返回节点 id。对应 Java: `NodeSimpleVO#getNodeId`。
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 返回节点 ID。对应 Java: `NodeSimpleVO#getNodeId`。
    #[must_use]
    pub fn get_node_id(&self) -> &str {
        self.node_id()
    }

    /// 设置节点 id。参数 `node_id` 为规则源中的节点标识。
    /// 对应 Java: `NodeSimpleVO#setNodeId`。
    pub fn set_node_id(&mut self, node_id: impl Into<String>) {
        self.node_id = node_id.into();
    }

    /// 返回节点类型 code。对应 Java: `NodeSimpleVO#getType`。
    #[must_use]
    pub fn node_type(&self) -> &str {
        &self.node_type
    }

    /// 返回节点类型 code。对应 Java: `NodeSimpleVO#getType`。
    #[must_use]
    pub fn get_type(&self) -> &str {
        self.node_type()
    }

    /// 设置节点类型 code。参数 `node_type` 对应 Java 规则的 `type` 字段。
    /// 对应 Java: `NodeSimpleVO#setType`。
    pub fn set_type(&mut self, node_type: impl Into<String>) {
        self.node_type = node_type.into();
    }

    /// 返回节点名称；未配置时返回空字符串。对应 Java: `NodeSimpleVO#getName`。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回节点名称；未配置时返回空字符串。
    ///
    /// 对应 Java: `NodeSimpleVO#getName`。
    #[must_use]
    pub fn get_name(&self) -> &str {
        self.name()
    }

    /// 设置节点名称。参数 `name` 为可选的人类可读名称。
    /// 对应 Java: `NodeSimpleVO#setName`。
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// 返回脚本语言；键中未配置时返回 `None`。
    /// 对应 Java: `NodeSimpleVO#getLanguage`。
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// 返回脚本语言；未配置时返回 `None`，对应 Java `null`。
    ///
    /// 对应 Java: `NodeSimpleVO#getLanguage`。
    #[must_use]
    pub fn get_language(&self) -> Option<&str> {
        self.language()
    }

    /// 设置脚本语言。参数 `language` 为脚本引擎名称。
    /// 对应 Java: `NodeSimpleVO#setLanguage`。
    pub fn set_language(&mut self, language: impl Into<String>) {
        self.language = Some(language.into());
    }

    /// 返回节点是否启用；未配置时默认为 `true`。
    /// 对应 Java: `NodeSimpleVO#getEnable`。
    #[must_use]
    pub fn enable(&self) -> bool {
        self.enable
    }

    /// 返回节点是否启用；默认值为 true。
    ///
    /// 对应 Java: `NodeSimpleVO#getEnable`。
    #[must_use]
    pub fn get_enable(&self) -> bool {
        self.enable()
    }

    /// 设置节点启用状态。参数 `enable` 决定生成 XML 后是否装载该节点。
    /// 对应 Java: `NodeSimpleVO#setEnable`。
    pub fn set_enable(&mut self, enable: bool) {
        self.enable = enable;
    }

    /// 返回规则源取得的脚本正文。对应 Java: `NodeSimpleVO#getScript`。
    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    /// 返回规则源取得的脚本正文；未写入时返回 `None`。
    ///
    /// 对应 Java: `NodeSimpleVO#getScript`。
    #[must_use]
    pub fn get_script(&self) -> Option<&str> {
        self.script()
    }

    /// 写入规则源取得的脚本正文。对应 Java: `NodeSimpleVO#setScript`。
    pub fn set_script(&mut self, script: impl Into<String>) {
        self.script = Some(script.into());
    }
}

/// 将规则插件使用的冒号键转换为脚本节点描述。
///
/// 对应 Java: `com.yomahub.liteflow.parser.helper.NodeConvertHelper`。
pub struct NodeConvertHelper;

impl NodeConvertHelper {
    /// 解析 `id:type[:name[:language[:enable]]]` 形式的脚本键。
    ///
    /// Java 正则只保留与另一个非空段通过单个冒号相邻的段；这里逐段复现该
    /// 行为，因此没有完整 `id:type` 的输入返回 `None`。
    /// 对应 Java: `NodeConvertHelper#convert`。
    #[must_use]
    pub fn convert(script_key: &str) -> Option<NodeSimpleVO> {
        let parts: Vec<&str> = script_key.split(':').collect();
        let match_items: Vec<&str> = parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| {
                if part.is_empty() {
                    return None;
                }
                let has_left = index > 0 && !parts[index - 1].is_empty();
                let has_right = index + 1 < parts.len() && !parts[index + 1].is_empty();
                (has_left || has_right).then_some(*part)
            })
            .collect();

        if match_items.len() < 2 {
            return None;
        }

        let mut node = NodeSimpleVO::new(match_items[0], match_items[1]);
        if let Some(name) = match_items.get(2) {
            node.set_name(*name);
        }
        if let Some(language) = match_items.get(3) {
            node.set_language(*language);
        }
        if let Some(enable) = match_items.get(4) {
            node.set_enable(enable.eq_ignore_ascii_case("true"));
        }
        Some(node)
    }
}
