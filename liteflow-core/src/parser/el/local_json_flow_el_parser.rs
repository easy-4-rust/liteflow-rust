//! 本地 JSON/JSON-EL 文件解析器。

use std::path::Path;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::el::JsonFlowElParser;
use crate::spi::holder::PathContentParserHolder;

/// 通过 `PathContentParser` SPI 读取本地 JSON 规则后执行统一解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.LocalJsonFlowELParser`。
pub struct LocalJsonFlowElParser {
    parser: JsonFlowElParser,
}

impl LocalJsonFlowElParser {
    /// 使用目标流程总线创建本地 JSON 解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            parser: JsonFlowElParser::new(bus),
        }
    }

    /// 读取路径列表并解析 JSON EL 规则。
    ///
    /// - `path_list`：本地文件或 `PathContentParser` 支持的资源路径列表。
    /// - 返回：成功装载到 `FlowBus` 的 Chain ID 列表。
    ///
    /// 路径读取失败或 JSON 规则非法时返回对应 `LiteflowError`。对应 Java:
    /// `LocalJsonFlowELParser#parseMain`。
    pub fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        let contents = PathContentParserHolder::load_context_aware().parse_content(path_list)?;
        self.parse(&contents)
    }
}

impl FlowParser for LocalJsonFlowElParser {
    /// 读取路径列表并解析 JSON 规则。
    ///
    /// 参数 `path_list` 对应 Java `pathList`。
    /// 对应 Java: `LocalJsonFlowELParser#parseMain`。
    fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        LocalJsonFlowElParser::parse_main(self, path_list)
    }

    /// 解析已经读取的 JSON 规则文本。
    fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.parser.parse(content_list)
    }
}

/// 兼容原有 Rust API：解析一段 JSON 规则文本。
pub fn load_json_str(bus: &FlowBus, json: &str) -> LFResult<Vec<String>> {
    LocalJsonFlowElParser::new(bus.clone()).parse(&[json.to_string()])
}

/// 兼容原有 Rust API：解析一个本地 JSON 规则文件。
pub fn load_json_file(bus: &FlowBus, path: impl AsRef<Path>) -> LFResult<Vec<String>> {
    let path_list = vec![path.as_ref().to_string_lossy().into_owned()];
    LocalJsonFlowElParser::new(bus.clone()).parse_main(&path_list)
}
