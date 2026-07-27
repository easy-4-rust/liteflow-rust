//! 自定义内容源 YML EL 解析器。

use std::sync::Arc;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::el::YmlFlowElParser;

/// 以闭包承接 Java `parseCustom()` 的自定义 YML 规则解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.el.ClassYmlFlowELParser`。
pub struct ClassYmlFlowElParser {
    parser: YmlFlowElParser,
    content_provider: Arc<dyn Fn() -> LFResult<String> + Send + Sync>,
}

impl ClassYmlFlowElParser {
    /// 使用流程总线与自定义内容提供器创建解析器。
    #[must_use]
    pub fn new(
        bus: FlowBus,
        content_provider: Arc<dyn Fn() -> LFResult<String> + Send + Sync>,
    ) -> Self {
        Self {
            parser: YmlFlowElParser::new(bus),
            content_provider,
        }
    }

    /// 获取自定义规则文本。
    ///
    /// 对应 Java: `ClassYmlFlowELParser#parseCustom`。
    pub fn parse_custom(&self) -> LFResult<String> {
        (self.content_provider)()
    }

    /// 调用自定义内容源并解析 YML EL 规则。
    ///
    /// - `path_list`：对应 Java `pathList`；自定义类解析器按 Java 语义忽略路径。
    /// - 返回：成功装载到 `FlowBus` 的 Chain ID 列表。
    ///
    /// 内容提供器失败或 YML 规则非法时返回对应 `LiteflowError`。对应 Java:
    /// `ClassYmlFlowELParser#parseMain`。
    pub fn parse_main(&self, _path_list: &[String]) -> LFResult<Vec<String>> {
        let content = self.parse_custom()?;
        self.parse(&[content])
    }
}

impl FlowParser for ClassYmlFlowElParser {
    /// 忽略路径参数，调用自定义内容源并解析。
    ///
    /// 对应 Java: `ClassYmlFlowELParser#parseMain`。
    fn parse_main(&self, path_list: &[String]) -> LFResult<Vec<String>> {
        ClassYmlFlowElParser::parse_main(self, path_list)
    }

    /// 解析已经读取的 YML 规则文本。
    fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        self.parser.parse(content_list)
    }
}
