//! YML EL 规则解析器。

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::BaseYmlFlowParser;

/// YML EL 解析器，复用 YML 基类完成格式转换、节点、链路与继承解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.YmlFlowELParser`。
#[derive(Clone)]
pub struct YmlFlowElParser {
    base_parser: BaseYmlFlowParser,
}

impl YmlFlowElParser {
    /// 使用目标流程总线创建 YML EL 解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            base_parser: BaseYmlFlowParser::new(bus),
        }
    }

    /// 解析 YML 规则文本列表并返回成功装载的 chain id。
    ///
    /// 对应 Java 父类: `BaseYmlFlowParser#parse`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.base_parser.parse(content_list)
    }
}
