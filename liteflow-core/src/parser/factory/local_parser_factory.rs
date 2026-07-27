//! 本地规则文件解析器工厂。

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::el::{LocalJsonFlowElParser, LocalXmlFlowElParser, LocalYmlFlowElParser};
use crate::parser::factory::FlowParserFactory;

/// 为本地 JSON/XML/YML 规则路径创建对应解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.factory.LocalParserFactory`。
#[derive(Clone)]
pub struct LocalParserFactory {
    bus: FlowBus,
}

impl LocalParserFactory {
    /// 使用目标流程总线创建本地解析器工厂。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self { bus }
    }

    /// 创建本地 JSON EL 解析器。
    ///
    /// - `path`：对应 Java `path`；工厂只选择解析器类型，实际读取发生在
    ///   `LocalJsonFlowElParser#parse_main`。
    /// - 返回：绑定当前 `FlowBus` 的 JSON 解析器。
    ///
    /// 对应 Java: `LocalParserFactory#createJsonELParser`。
    pub fn create_json_el_parser(&self, _path: &str) -> LFResult<Box<dyn FlowParser>> {
        Ok(Box::new(LocalJsonFlowElParser::new(self.bus.clone())))
    }

    /// 创建本地 XML EL 解析器。
    ///
    /// - `path`：对应 Java `path`；路径由解析器执行阶段读取。
    /// - 返回：绑定当前 `FlowBus` 的 XML 解析器。
    ///
    /// 对应 Java: `LocalParserFactory#createXmlELParser`。
    pub fn create_xml_el_parser(&self, _path: &str) -> LFResult<Box<dyn FlowParser>> {
        Ok(Box::new(LocalXmlFlowElParser::new(self.bus.clone())))
    }

    /// 创建本地 YML EL 解析器。
    ///
    /// - `path`：对应 Java `path`；路径由解析器执行阶段读取。
    /// - 返回：绑定当前 `FlowBus` 的 YML 解析器。
    ///
    /// 对应 Java: `LocalParserFactory#createYmlELParser`。
    pub fn create_yml_el_parser(&self, _path: &str) -> LFResult<Box<dyn FlowParser>> {
        Ok(Box::new(LocalYmlFlowElParser::new(self.bus.clone())))
    }
}

impl FlowParserFactory for LocalParserFactory {
    /// 创建本地 JSON EL 解析器。
    fn create_json_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>> {
        LocalParserFactory::create_json_el_parser(self, path)
    }

    /// 创建本地 XML EL 解析器。
    fn create_xml_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>> {
        LocalParserFactory::create_xml_el_parser(self, path)
    }

    /// 创建本地 YML EL 解析器。
    fn create_yml_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>> {
        LocalParserFactory::create_yml_el_parser(self, path)
    }
}
