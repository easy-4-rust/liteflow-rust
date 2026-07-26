//! 自定义内容源 XML EL 解析器。

use std::sync::Arc;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::el::XmlFlowElParser;

/// 以闭包承接 Java `parseCustom()` 的自定义 XML 规则解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.ClassXmlFlowELParser`。
pub struct ClassXmlFlowElParser {
    parser: XmlFlowElParser,
    content_provider: Arc<dyn Fn() -> LFResult<String> + Send + Sync>,
}

impl ClassXmlFlowElParser {
    /// 使用流程总线与自定义内容提供器创建解析器。
    #[must_use]
    pub fn new(
        bus: FlowBus,
        content_provider: Arc<dyn Fn() -> LFResult<String> + Send + Sync>,
    ) -> Self {
        Self {
            parser: XmlFlowElParser::new(bus),
            content_provider,
        }
    }

    /// 获取自定义规则文本。
    ///
    /// 对应 Java: `ClassXmlFlowELParser#parseCustom`。
    pub fn parse_custom(&self) -> LFResult<String> {
        (self.content_provider)()
    }
}

impl FlowParser for ClassXmlFlowElParser {
    /// 忽略路径参数，调用自定义内容源并解析。
    ///
    /// 对应 Java: `ClassXmlFlowELParser#parseMain`。
    fn parse_main(&self, _path_list: &[String]) -> LFResult<Vec<String>> {
        let content = self.parse_custom()?;
        self.parse(&[content])
    }

    /// 解析已经读取的 XML 规则文本。
    fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.parser.parse(content_list)
    }
}
