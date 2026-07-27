//! XML 规则解析器公共实现。

use std::collections::HashSet;

use crate::exception::LFResult;
use crate::flow::flow_bus::FlowBus;
use crate::parser::RuleDefinitionPlan;
use crate::parser::helper::ParserHelper;

/// 承载 XML/XML-EL 解析器共享的节点与链路解析逻辑。
///
/// 多份 XML 文本共享同一批中间链定义，最后统一解析继承并原子写入
/// `FlowBus`，避免跨文件父子链被拆开处理。
///
/// 对应 Java: `com.yomahub.liteflow.parser.base.BaseXmlFlowParser`。
#[derive(Clone)]
pub struct BaseXmlFlowParser {
    bus: FlowBus,
}

impl BaseXmlFlowParser {
    /// 使用目标流程总线创建解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self { bus }
    }

    /// 解析 XML 文本列表并返回成功装载的 chain id。
    ///
    /// 对应 Java: `BaseXmlFlowParser#parse(List<String>)`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        if content_list.is_empty() {
            return Ok(Vec::new());
        }

        self.collect(content_list)?.build_all(&self.bus)
    }

    /// 只读取 XML 节点与链定义，不创建 Chain 或编译脚本。
    ///
    /// 对应 Java `PARSE_ONE_ON_FIRST_EXEC` 的启动期定义收集阶段。
    pub fn collect(&self, content_list: &[String]) -> LFResult<RuleDefinitionPlan> {
        let mut plan = RuleDefinitionPlan::new();
        ParserHelper::parse_node_document(content_list, &mut plan)?;
        ParserHelper::parse_chain_document(content_list, &mut HashSet::new(), &mut plan)?;
        Ok(plan)
    }
}
