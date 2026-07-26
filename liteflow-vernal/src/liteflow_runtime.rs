//! 对应 Java: `FlowExecutor` + `LiteflowExecutorInit` 的 Vernal 托管对象。

use liteflow_core::{FlowBus, LiteflowResponse, rule};
use serde_json::Value;
use vernal_context::{Lifecycle, LifecycleFuture};

use crate::{LiteflowConfig, LiteflowRuleFormat, LiteflowVernalError};

/// 由 Vernal 管理生命周期的 LiteFlow 运行时。
pub struct LiteflowRuntime {
    flow_bus: FlowBus,
    config: LiteflowConfig,
}

impl LiteflowRuntime {
    /// 创建运行时。规则解析由 Vernal `initialize` 阶段统一触发。
    #[must_use]
    pub fn new(flow_bus: FlowBus, config: LiteflowConfig) -> Self {
        Self { flow_bus, config }
    }

    /// 返回共享 FlowBus。
    #[must_use]
    pub fn flow_bus(&self) -> &FlowBus {
        &self.flow_bus
    }

    /// 返回冻结后的 LiteFlow 配置。
    #[must_use]
    pub fn config(&self) -> &LiteflowConfig {
        &self.config
    }

    /// 执行链路；对应 Java `FlowExecutor#execute2Resp`。
    pub async fn execute(&self, chain_id: &str, input: Value) -> LiteflowResponse {
        self.flow_bus.execute_with_data(chain_id, input).await
    }

    /// 使用显式 request id 执行链路。
    pub async fn execute_with_rid(
        &self,
        chain_id: &str,
        input: Value,
        request_id: impl Into<String>,
    ) -> LiteflowResponse {
        self.flow_bus
            .execute_with_rid(chain_id, input, request_id)
            .await
    }

    fn initialize_rule(&self) -> Result<(), LiteflowVernalError> {
        if !self.config.enable {
            return Ok(());
        }
        if self.config.rule_source.is_some() && self.config.inline_rule.is_some() {
            return Err(LiteflowVernalError::ConflictingRuleSource);
        }
        match (
            self.config.rule_source.as_deref(),
            self.config.inline_rule.as_deref(),
            self.config.rule_format,
        ) {
            (Some(source), None, LiteflowRuleFormat::Json) => {
                rule::load_json_file(&self.flow_bus, source)
            }
            (Some(source), None, LiteflowRuleFormat::Xml) => {
                rule::load_xml_file(&self.flow_bus, source)
            }
            (Some(source), None, LiteflowRuleFormat::Yml) => {
                rule::load_yml_file(&self.flow_bus, source)
            }
            (None, Some(text), LiteflowRuleFormat::Json) => {
                rule::load_json_str(&self.flow_bus, text)
            }
            (None, Some(text), LiteflowRuleFormat::Xml) => rule::load_xml_str(&self.flow_bus, text),
            (None, Some(text), LiteflowRuleFormat::Yml) => rule::load_yml_str(&self.flow_bus, text),
            (None, None, _) => return Ok(()),
            (Some(_), Some(_), _) => unreachable!("conflict checked above"),
        }
        .map(|_| ())
        .map_err(|error| LiteflowVernalError::RuleInitialization(error.to_string()))
    }
}

impl Lifecycle for LiteflowRuntime {
    fn initialize(&self) -> LifecycleFuture<'_> {
        Box::pin(async move {
            self.initialize_rule()
                .map_err(|error| Box::new(error) as vernal_core::BoxError)
        })
    }
}
