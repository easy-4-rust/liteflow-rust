use crate::parser::helper::NodeSimpleVO;

/// 规则插件链路键的解析结果。
///
/// Java 中该对象是 `RuleParsePluginUtil` 的静态内部类，因此与主对象保留在
/// 同一文件。`enable` 只保存规范化后的 `"true"` 或 `"false"`。
///
/// 对应 Java: `com.yomahub.liteflow.util.RuleParsePluginUtil.ChainDto`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainDto {
    id: String,
    enable: String,
}

impl ChainDto {
    /// 仅使用链路 id 创建默认启用的描述。
    /// 对应 Java: `ChainDto#ChainDto(String)`。
    #[must_use]
    pub fn new(chain_id: impl Into<String>) -> Self {
        Self::with_enable(chain_id, None)
    }

    /// 使用链路 id 与启用文本创建描述；空白或忽略大小写的 `true` 表示启用。
    /// 对应 Java: `ChainDto#ChainDto(String,String)`。
    #[must_use]
    pub fn with_enable(chain_id: impl Into<String>, enable: Option<&str>) -> Self {
        let enabled = enable
            .is_none_or(|value| value.trim().is_empty() || value.eq_ignore_ascii_case("true"));
        Self {
            id: chain_id.into(),
            enable: enabled.to_string(),
        }
    }

    /// 返回链路 id。对应 Java: `ChainDto#getId`。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回链路 id。
    ///
    /// 返回值来自规则键中冒号前的标识；非标准多段键会保留完整原文。
    /// 对应 Java: `ChainDto#getId`。
    #[must_use]
    pub fn get_id(&self) -> &str {
        self.id()
    }

    /// 返回规范化后的启用文本。对应 Java: `ChainDto#getEnable`。
    #[must_use]
    pub fn enable(&self) -> &str {
        &self.enable
    }

    /// 返回规范化后的启用文本。
    ///
    /// 返回值只可能是 `"true"` 或 `"false"`。
    /// 对应 Java: `ChainDto#getEnable`。
    #[must_use]
    pub fn get_enable(&self) -> &str {
        self.enable()
    }

    /// 返回链路是否启用。对应 Java: `ChainDto#isEnable`。
    #[must_use]
    pub fn is_enable(&self) -> bool {
        self.enable.eq_ignore_ascii_case("true")
    }

    /// 返回链路是否禁用。对应 Java: `ChainDto#isDisable`。
    #[must_use]
    pub fn is_disable(&self) -> bool {
        !self.is_enable()
    }

    /// 将 EL 正文转换为插件汇总规则所需的 `<chain>` XML。
    /// 对应 Java: `ChainDto#toElXml`。
    #[must_use]
    pub fn to_el_xml(&self, el_content: &str) -> String {
        format!(
            "<chain id=\"{}\" enable=\"{}\">{el_content}</chain>",
            self.get_id(),
            self.get_enable()
        )
    }
}

/// 分布式规则插件共享的键解析与 XML 片段生成工具。
///
/// 对应 Java: `com.yomahub.liteflow.util.RuleParsePluginUtil`。
pub struct RuleParsePluginUtil;

impl RuleParsePluginUtil {
    /// 解析 `chain_id:enable` 键；不是恰好两段时按默认启用的完整 id 处理。
    /// 对应 Java: `RuleParsePluginUtil#parseChainKey`。
    #[must_use]
    pub fn parse_chain_key(chain_key: &str) -> ChainDto {
        let parts: Vec<&str> = chain_key.split(':').collect();
        if parts.len() == 2 {
            ChainDto::with_enable(parts[0], Some(parts[1]))
        } else {
            ChainDto::new(chain_key)
        }
    }

    /// 将脚本节点描述转换为 `<node>` XML；语言非空时才输出 `language` 属性。
    /// 对应 Java: `RuleParsePluginUtil#toScriptXml`。
    #[must_use]
    pub fn to_script_xml(node: &NodeSimpleVO) -> String {
        let script = node.script().unwrap_or_default();
        match node
            .language()
            .filter(|language| !language.trim().is_empty())
        {
            Some(language) => format!(
                "<node id=\"{}\" name=\"{}\" type=\"{}\" language=\"{}\" enable=\"{}\"><![CDATA[{}]]></node>",
                node.node_id(),
                node.name(),
                node.node_type(),
                language,
                node.enable(),
                script
            ),
            None => format!(
                "<node id=\"{}\" name=\"{}\" type=\"{}\" enable=\"{}\"><![CDATA[{}]]></node>",
                node.node_id(),
                node.name(),
                node.node_type(),
                node.enable(),
                script
            ),
        }
    }

    /// 解析 `id:enable` 更新键并返回 `(是否启用, id)`。
    ///
    /// 非恰好两段时与 Java 一致，按默认启用处理并保留完整键为 id。
    /// 对应 Java: `RuleParsePluginUtil#parseIdKey`。
    #[must_use]
    pub fn parse_id_key(id_key: &str) -> (bool, String) {
        let parts: Vec<&str> = id_key.split(':').collect();
        if parts.len() == 2 {
            (parts[1].eq_ignore_ascii_case("true"), parts[0].to_string())
        } else {
            (true, id_key.to_string())
        }
    }
}
