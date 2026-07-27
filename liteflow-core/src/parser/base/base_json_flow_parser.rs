//! JSON 规则解析器公共实现。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::parser::RuleDefinitionPlan;
use crate::parser::helper::ParserHelper;
use serde_json::Value;
use std::collections::HashSet;

/// 承载 JSON/JSON-EL 解析器共享的节点与链路解析逻辑。
///
/// 多份规则文本先统一收集 `ChainDef`，再一次性处理继承关系，保证父链与子链
/// 分布在不同文件时仍可正确解析。
///
/// 对应 Java: `com.yomahub.liteflow.parser.base.BaseJsonFlowParser`。
#[derive(Clone)]
pub struct BaseJsonFlowParser {
    bus: FlowBus,
}

impl BaseJsonFlowParser {
    /// 使用目标流程总线创建解析器。
    ///
    /// 参数 `bus` 接收解析后节点和链路；返回可复用的解析器实例。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self { bus }
    }

    /// 解析 JSON 文本列表。
    ///
    /// 空列表与 Java 基类一致，视为无需加载并返回空结果。
    /// 对应 Java: `BaseJsonFlowParser#parse(List<String>)`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        if content_list.is_empty() {
            return Ok(Vec::new());
        }

        self.collect(content_list)?.build_all(&self.bus)
    }

    /// 只解析格式并保存节点、链定义，不创建 Chain 或编译脚本。
    ///
    /// 对应 Java `PARSE_ONE_ON_FIRST_EXEC` 启动阶段预装载 Chain 定义的行为。
    pub fn collect(&self, content_list: &[String]) -> LFResult<RuleDefinitionPlan> {
        let mut values = Vec::with_capacity(content_list.len());
        for content in content_list {
            let value = serde_json::from_str(content)
                .map_err(|error| LiteflowError::Rule(format!("invalid json: {error}")))?;
            values.push(value);
        }
        self.collect_values(&values)
    }

    /// 收集已经转换为 JSON Value 的规则列表，供 YML 延迟解析复用。
    pub(crate) fn collect_values(&self, values: &[Value]) -> LFResult<RuleDefinitionPlan> {
        if values.is_empty() {
            return Ok(RuleDefinitionPlan::new());
        }

        let mut plan = RuleDefinitionPlan::new();
        ParserHelper::parse_node_json(values, &mut plan)?;
        ParserHelper::parse_chain_json(values, &mut HashSet::new(), &mut plan)?;
        Ok(plan)
    }
}
