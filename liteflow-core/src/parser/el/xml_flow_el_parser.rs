//! XML EL 规则解析器。

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::BaseXmlFlowParser;

/// XML EL 解析器，复用 XML 基类完成节点、链路与继承解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.XmlFlowELParser`。
#[derive(Clone)]
pub struct XmlFlowElParser {
    base_parser: BaseXmlFlowParser,
}

impl XmlFlowElParser {
    /// 使用目标流程总线创建 XML EL 解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            base_parser: BaseXmlFlowParser::new(bus),
        }
    }

    /// 解析 XML 规则文本列表并返回成功装载的 chain id。
    ///
    /// 对应 Java 父类: `BaseXmlFlowParser#parse`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.base_parser.parse(content_list)
    }
}
