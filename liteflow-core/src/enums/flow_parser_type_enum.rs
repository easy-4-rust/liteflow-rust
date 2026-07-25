//! 对应 com.yomahub.liteflow.enums.FlowParserTypeEnum：
//! 规则解析器类型（xml/yml/json 普通格式与 el_xml/el_json/el_yml EL 格式）。
//! Java 每个枚举携带 type 与 name 两个字符串字段。

/// 规则解析器类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowParserTypeEnum {
    Xml,
    Yml,
    Json,
    ElXml,
    ElJson,
    ElYml,
}

impl FlowParserTypeEnum {
    /// getType()
    pub fn get_type(&self) -> &'static str {
        match self {
            Self::Xml => "xml",
            Self::Yml => "yml",
            Self::Json => "json",
            Self::ElXml => "el_xml",
            Self::ElJson => "el_json",
            Self::ElYml => "el_yml",
        }
    }
    /// getName()
    pub fn get_name(&self) -> &'static str {
        self.get_type()
    }
}
