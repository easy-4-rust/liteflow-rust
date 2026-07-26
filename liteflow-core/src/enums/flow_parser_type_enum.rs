//! 对应 com.yomahub.liteflow.enums.FlowParserTypeEnum：
//! 规则解析器类型（xml/yml/json 普通格式与 el_xml/el_json/el_yml EL 格式）。
//! Java 每个枚举携带 type 与 name 两个字符串字段。

/// 规则解析器类型枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FlowParserTypeEnum {
    /// 普通 XML 规则解析器，对应 Java `TYPE_XML`。
    #[serde(rename = "xml")]
    TypeXml,
    /// 普通 YML 规则解析器，对应 Java `TYPE_YML`。
    #[serde(rename = "yml")]
    TypeYml,
    /// 普通 JSON 规则解析器，对应 Java `TYPE_JSON`。
    #[serde(rename = "json")]
    TypeJson,
    /// EL XML 规则解析器，对应 Java `TYPE_EL_XML`。
    #[serde(rename = "el_xml")]
    TypeElXml,
    /// EL JSON 规则解析器，对应 Java `TYPE_EL_JSON`。
    #[serde(rename = "el_json")]
    TypeElJson,
    /// EL YML 规则解析器，对应 Java `TYPE_EL_YML`。
    #[serde(rename = "el_yml")]
    TypeElYml,
}

impl FlowParserTypeEnum {
    /// getType()
    pub fn get_type(&self) -> &'static str {
        match self {
            Self::TypeXml => "xml",
            Self::TypeYml => "yml",
            Self::TypeJson => "json",
            Self::TypeElXml => "el_xml",
            Self::TypeElJson => "el_json",
            Self::TypeElYml => "el_yml",
        }
    }
    /// getName()
    pub fn get_name(&self) -> &'static str {
        self.get_type()
    }

    /// 按 Java `type` 字段反查枚举。
    pub fn get_enum_by_type(parser_type: &str) -> Option<Self> {
        [
            Self::TypeXml,
            Self::TypeYml,
            Self::TypeJson,
            Self::TypeElXml,
            Self::TypeElJson,
            Self::TypeElYml,
        ]
        .into_iter()
        .find(|item| item.get_type() == parser_type)
    }
}
