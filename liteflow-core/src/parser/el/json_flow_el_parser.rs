//! JSON EL 规则解析器。

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::BaseJsonFlowParser;

/// JSON EL 解析器，复用 JSON 基类完成节点、链路与继承解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.JsonFlowELParser`。
#[derive(Clone)]
pub struct JsonFlowElParser {
    base_parser: BaseJsonFlowParser,
}

impl JsonFlowElParser {
    /// 使用目标流程总线创建 JSON EL 解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            base_parser: BaseJsonFlowParser::new(bus),
        }
    }

    /// 解析 JSON 规则文本列表并返回成功装载的 chain id。
    ///
    /// 对应 Java 父类: `BaseJsonFlowParser#parse`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.base_parser.parse(content_list)
    }
}
