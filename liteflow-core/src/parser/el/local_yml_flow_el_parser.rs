//! 本地 YML/YML-EL 文件解析器。

use std::path::Path;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::el::YmlFlowElParser;
use crate::spi::holder::PathContentParserHolder;

/// 通过 `PathContentParser` SPI 读取本地 YML 规则后执行统一解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.LocalYmlFlowELParser`。
pub struct LocalYmlFlowElParser {
    parser: YmlFlowElParser,
}

impl LocalYmlFlowElParser {
    /// 使用目标流程总线创建本地 YML 解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            parser: YmlFlowElParser::new(bus),
        }
    }

    /// 读取路径列表并解析 YML EL 规则。
    ///
    /// - `path_list`：本地文件或 `PathContentParser` 支持的资源路径列表。
    /// - 返回：成功装载到 `FlowBus` 的 Chain ID 列表。
    ///
    /// 路径读取失败或 YML 规则非法时返回对应 `LiteflowError`。对应 Java:
    /// `LocalYmlFlowELParser#parseMain`。
    pub fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        let contents = PathContentParserHolder::load_context_aware().parse_content(path_list)?;
        self.parse(&contents)
    }
}

impl FlowParser for LocalYmlFlowElParser {
    /// 读取路径列表并解析 YML 规则。
    ///
    /// 参数 `path_list` 对应 Java `pathList`。
    /// 对应 Java: `LocalYmlFlowELParser#parseMain`。
    fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        LocalYmlFlowElParser::parse_main(self, path_list)
    }

    /// 解析已经读取的 YML 规则文本。
    fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.parser.parse(content_list)
    }
}

/// 兼容原有 Rust API：解析一段 YML 规则文本。
pub fn load_yml_str(bus: &FlowBus, yml: &str) -> LFResult<Vec<String>> {
    LocalYmlFlowElParser::new(bus.clone()).parse(&[yml.to_string()])
}

/// 兼容原有 Rust API：解析一个本地 YML 规则文件。
pub fn load_yml_file(bus: &FlowBus, path: impl AsRef<Path>) -> LFResult<Vec<String>> {
    let path_list = vec![path.as_ref().to_string_lossy().into_owned()];
    LocalYmlFlowElParser::new(bus.clone()).parse_main(&path_list)
}
