//! YML 规则解析器公共实现。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::parser::RuleDefinitionPlan;
use crate::parser::base::BaseJsonFlowParser;

/// 承载 YML/YML-EL 解析器共享的转换与解析逻辑。
///
/// YAML 先由 `serde_yaml_ng` 转成 `serde_json::Value`，随后复用 JSON
/// 基类的节点、链路、继承和原子构建逻辑，对应 Java 中 SnakeYAML + Jackson。
///
/// 对应 Java: `com.yomahub.liteflow.parser.base.BaseYmlFlowParser`。
#[derive(Clone)]
pub struct BaseYmlFlowParser {
    bus: FlowBus,
    json_parser: BaseJsonFlowParser,
}

impl BaseYmlFlowParser {
    /// 使用目标流程总线创建解析器。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            json_parser: BaseJsonFlowParser::new(bus.clone()),
            bus,
        }
    }

    /// 解析 YML 文本列表并返回成功装载的 chain id。
    ///
    /// 对应 Java: `BaseYmlFlowParser#parse(List<String>)`。
    pub fn parse(&self, content_list: &[String]) -> LFResult<Vec<String>> {
        if content_list.is_empty() {
            return Ok(Vec::new());
        }

        self.collect(content_list)?.build_all(&self.bus)
    }

    /// 只解析 YAML 并保存格式无关的规则计划，不构建 Chain 或脚本节点。
    pub fn collect(&self, content_list: &[String]) -> LFResult<RuleDefinitionPlan> {
        let mut values = Vec::with_capacity(content_list.len());
        for content in content_list {
            let yaml_value: serde_yaml_ng::Value = serde_yaml_ng::from_str(content)
                .map_err(|error| LiteflowError::Rule(format!("invalid yml: {error}")))?;
            let json_value = serde_json::to_value(yaml_value)
                .map_err(|error| LiteflowError::Rule(format!("yml convert error: {error}")))?;
            values.push(json_value);
        }
        self.json_parser.collect_values(&values)
    }
}
