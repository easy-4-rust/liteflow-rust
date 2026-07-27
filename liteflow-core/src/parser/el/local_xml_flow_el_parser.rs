//! 本地 XML/XML-EL 文件解析器。

use std::path::Path;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::el::XmlFlowElParser;
use crate::spi::holder::PathContentParserHolder;

/// 通过 `PathContentParser` SPI 读取本地 XML 规则后执行统一解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.LocalXmlFlowELParser`。
pub struct LocalXmlFlowElParser {
    parser: XmlFlowElParser,
}

impl LocalXmlFlowElParser {
    /// 使用目标流程总线创建本地 XML 解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            parser: XmlFlowElParser::new(bus),
        }
    }

    /// 读取路径列表并解析 XML EL 规则。
    ///
    /// - `path_list`：本地文件或 `PathContentParser` 支持的资源路径列表。
    /// - 返回：成功装载到 `FlowBus` 的 Chain ID 列表。
    ///
    /// 路径读取失败或 XML 规则非法时返回对应 `LiteflowError`。对应 Java:
    /// `LocalXmlFlowELParser#parseMain`。
    pub fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        let contents =
            PathContentParserHolder::load_path_content_parser().parse_content(path_list)?;
        self.parse(&contents)
    }
}

impl FlowParser for LocalXmlFlowElParser {
    /// 读取路径列表并解析 XML 规则。
    ///
    /// 参数 `path_list` 对应 Java `pathList`。
    /// 对应 Java: `LocalXmlFlowELParser#parseMain`。
    fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        LocalXmlFlowElParser::parse_main(self, path_list)
    }

    /// 解析已经读取的 XML 规则文本。
    fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.parser.parse(content_list)
    }
}

/// 兼容原有 Rust API：解析一段 XML 规则文本。
pub fn load_xml_str(bus: &FlowBus, xml: &str) -> LFResult<Vec<String>> {
    LocalXmlFlowElParser::new(bus.clone()).parse(&[xml.to_string()])
}

/// 兼容原有 Rust API：解析一个本地 XML 规则文件。
pub fn load_xml_file(bus: &FlowBus, path: impl AsRef<Path>) -> LFResult<Vec<String>> {
    let path_list = vec![path.as_ref().to_string_lossy().into_owned()];
    LocalXmlFlowElParser::new(bus.clone()).parse_main(&path_list)
}
