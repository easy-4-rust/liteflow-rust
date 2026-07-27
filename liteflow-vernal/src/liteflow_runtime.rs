//! 对应 Java: `FlowExecutor` + `LiteflowExecutorInit` 的 Vernal 托管对象。

use std::sync::Mutex;
use std::time::Duration;

use liteflow_core::lifecycle::ChainCacheLifeCycle;
use liteflow_core::monitor::MonitorTimeTask;
use liteflow_core::parser::{
    BaseJsonFlowParser, BaseXmlFlowParser, BaseYmlFlowParser, RuleDefinitionPlan,
};
use liteflow_core::spi::PathContentParserHolder;
use liteflow_core::{FlowBus, LiteflowResponse};
use serde_json::Value;
use vernal_context::{Lifecycle, LifecycleFuture};

use crate::rule_initialization_state::RuleInitializationState;
use crate::{LiteflowConfig, LiteflowParseMode, LiteflowRuleFormat, LiteflowVernalError};

/// 由 Vernal 管理生命周期的 LiteFlow 运行时。
pub struct LiteflowRuntime {
    flow_bus: FlowBus,
    config: LiteflowConfig,
    rule_state: Mutex<RuleInitializationState>,
    chain_cache: Mutex<Option<std::sync::Arc<ChainCacheLifeCycle>>>,
    monitor_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl LiteflowRuntime {
    /// 创建运行时。规则解析由 Vernal `initialize` 阶段统一触发。
    #[must_use]
    pub fn new(flow_bus: FlowBus, config: LiteflowConfig) -> Self {
        // Java MonitorBus 构造函数从 LiteflowConfig 读取 queueLimit；Rust 在托管
        // 运行时创建时完成同样接线，确保首条统计记录就使用配置容量。
        flow_bus.monitor().set_queue_limit(config.queue_limit);
        Self {
            flow_bus,
            config,
            rule_state: Mutex::new(RuleInitializationState::Uninitialized),
            chain_cache: Mutex::new(None),
            monitor_task: Mutex::new(None),
        }
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

    /// 返回监控定时任务当前是否处于运行状态。
    ///
    /// 该诊断入口用于宿主健康检查；对应 Java `MonitorBus` 内部调度器是否已启动。
    #[must_use]
    pub fn is_monitor_task_running(&self) -> bool {
        self.monitor_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .is_some_and(|task| !task.is_finished())
    }

    /// 执行链路；对应 Java `FlowExecutor#execute2Resp`。
    pub async fn execute(&self, chain_id: &str, input: Value) -> LiteflowResponse {
        match self.try_execute(chain_id, input.clone()).await {
            Ok(response) => response,
            Err(error) => LiteflowResponse::initialization_failure(
                "rule-init-failed",
                chain_id,
                input,
                error.to_string(),
            ),
        }
    }

    /// 使用显式 request id 执行链路。
    pub async fn execute_with_rid(
        &self,
        chain_id: &str,
        input: Value,
        request_id: impl Into<String>,
    ) -> LiteflowResponse {
        let request_id = request_id.into();
        match self
            .try_execute_with_rid(chain_id, input.clone(), request_id.clone())
            .await
        {
            Ok(response) => response,
            Err(error) => LiteflowResponse::initialization_failure(
                request_id,
                chain_id,
                input,
                error.to_string(),
            ),
        }
    }

    /// 在执行前完成当前解析模式要求的规则初始化，并区分初始化错误与链执行失败。
    ///
    /// 对应 Java `FlowExecutor#doExecute` 中 `FlowBus.needInit()` 的首次执行分支。
    pub async fn try_execute(
        &self,
        chain_id: &str,
        input: Value,
    ) -> Result<LiteflowResponse, LiteflowVernalError> {
        self.ensure_rule_for_chain(chain_id)?;
        Ok(self.flow_bus.execute_with_data(chain_id, input).await)
    }

    /// 使用显式 request id 执行，并返回规则初始化错误。
    pub async fn try_execute_with_rid(
        &self,
        chain_id: &str,
        input: Value,
        request_id: impl Into<String>,
    ) -> Result<LiteflowResponse, LiteflowVernalError> {
        self.ensure_rule_for_chain(chain_id)?;
        Ok(self
            .flow_bus
            .execute_with_rid(chain_id, input, request_id)
            .await)
    }

    fn validate_rule_source(&self) -> Result<(), LiteflowVernalError> {
        if self.config.rule_source.is_some() && self.config.inline_rule.is_some() {
            return Err(LiteflowVernalError::ConflictingRuleSource);
        }
        Ok(())
    }

    /// 按配置装配 Chain 缓存生命周期。
    ///
    /// Java 仅允许在 `PARSE_ONE_ON_FIRST_EXEC` 下启用缓存；Rust 保持相同边界。
    /// 淘汰动作删除已物化 Chain，但保留 `RuleDefinitionPlan`，所以下次执行会重新
    /// 构建目标链及其依赖闭包。对应 Java `FlowExecutor#initChainCache`。
    fn ensure_chain_cache_initialized(&self) -> Result<(), LiteflowVernalError> {
        if !self.config.enable
            || !self.config.chain_cache_enabled
            || self.config.parse_mode != LiteflowParseMode::ParseOneOnFirstExec
        {
            return Ok(());
        }
        if self.config.chain_cache_capacity == 0 {
            return Err(LiteflowVernalError::RuleInitialization(
                "chain cache capacity must be greater than 0".to_string(),
            ));
        }

        let mut chain_cache = self
            .chain_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if chain_cache.is_none() {
            let lifecycle = std::sync::Arc::new(ChainCacheLifeCycle::new(
                self.config.chain_cache_capacity,
                self.flow_bus.chain_cache_cleaner(),
            ));
            self.flow_bus.register_chain_execute_hook(lifecycle.clone());
            *chain_cache = Some(lifecycle);
        }
        Ok(())
    }

    fn collect_rule_plan(&self) -> Result<RuleDefinitionPlan, LiteflowVernalError> {
        self.validate_rule_source()?;
        let contents = match (
            self.config.rule_source.as_deref(),
            self.config.inline_rule.as_deref(),
        ) {
            (Some(source), None) => PathContentParserHolder::load_path_content_parser()
                .parse_content(&[source.to_string()])
                .map_err(|error| LiteflowVernalError::RuleInitialization(error.to_string()))?,
            (None, Some(text)) => vec![text.to_string()],
            (None, None) => Vec::new(),
            (Some(_), Some(_)) => unreachable!("conflict checked above"),
        };
        let plan = match self.config.rule_format {
            LiteflowRuleFormat::Json => {
                BaseJsonFlowParser::new(self.flow_bus.clone()).collect(&contents)
            }
            LiteflowRuleFormat::Xml => {
                BaseXmlFlowParser::new(self.flow_bus.clone()).collect(&contents)
            }
            LiteflowRuleFormat::Yml => {
                BaseYmlFlowParser::new(self.flow_bus.clone()).collect(&contents)
            }
        };
        plan.map_err(|error| LiteflowVernalError::RuleInitialization(error.to_string()))
    }

    fn build_all_rules(&self) -> Result<(), LiteflowVernalError> {
        self.collect_rule_plan()?
            .build_all(&self.flow_bus)
            .map(|_| ())
            .map_err(|error| LiteflowVernalError::RuleInitialization(error.to_string()))
    }

    fn initialize_rule(&self) -> Result<(), LiteflowVernalError> {
        if !self.config.enable {
            *self
                .rule_state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                RuleInitializationState::Initialized;
            return Ok(());
        }
        self.validate_rule_source()?;
        let next_state = match self.config.parse_mode {
            LiteflowParseMode::ParseAllOnStart => {
                self.build_all_rules()?;
                RuleInitializationState::Initialized
            }
            LiteflowParseMode::ParseAllOnFirstExec => RuleInitializationState::Uninitialized,
            LiteflowParseMode::ParseOneOnFirstExec => {
                RuleInitializationState::Planned(self.collect_rule_plan()?)
            }
        };
        *self
            .rule_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = next_state;
        Ok(())
    }

    fn ensure_rule_for_chain(&self, chain_id: &str) -> Result<(), LiteflowVernalError> {
        if !self.config.enable {
            return Ok(());
        }
        self.ensure_chain_cache_initialized()?;
        let mut state = self
            .rule_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match &mut *state {
            RuleInitializationState::Initialized => Ok(()),
            RuleInitializationState::Failed(message) => {
                Err(LiteflowVernalError::RuleInitialization(message.clone()))
            }
            RuleInitializationState::Planned(plan) => plan
                .build_chain(&self.flow_bus, chain_id)
                .map_err(|error| LiteflowVernalError::RuleInitialization(error.to_string())),
            RuleInitializationState::Uninitialized => {
                let result = match self.config.parse_mode {
                    LiteflowParseMode::ParseOneOnFirstExec => {
                        let plan = self.collect_rule_plan()?;
                        let result = plan.build_chain(&self.flow_bus, chain_id);
                        *state = RuleInitializationState::Planned(plan);
                        result.map_err(|error| {
                            LiteflowVernalError::RuleInitialization(error.to_string())
                        })
                    }
                    LiteflowParseMode::ParseAllOnStart | LiteflowParseMode::ParseAllOnFirstExec => {
                        self.build_all_rules()
                    }
                };
                match result {
                    Ok(()) => {
                        if !matches!(*state, RuleInitializationState::Planned(_)) {
                            *state = RuleInitializationState::Initialized;
                        }
                        Ok(())
                    }
                    Err(error) => {
                        if !matches!(*state, RuleInitializationState::Planned(_)) {
                            *state = RuleInitializationState::Failed(error.to_string());
                        }
                        Err(error)
                    }
                }
            }
        }
    }

    /// 按 Java `MonitorBus(LiteflowConfig)` 构造语义启动周期监控。
    fn start_monitor_task(&self) {
        if !self.config.monitor_enable_log {
            return;
        }
        let mut task_guard = self
            .monitor_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if task_guard
            .as_ref()
            .is_some_and(|monitor_task| !monitor_task.is_finished())
        {
            return;
        }

        // MonitorTimeTask 持有共享 MonitorBus；Vernal stop 会终止并等待这个任务，
        // 避免容器关闭后仍遗留后台调度器。
        let monitor_task =
            std::sync::Arc::new(MonitorTimeTask::new(self.flow_bus.monitor().clone())).spawn(
                Duration::from_millis(self.config.delay),
                Duration::from_millis(self.config.period),
            );
        *task_guard = Some(monitor_task);
    }
}

impl Lifecycle for LiteflowRuntime {
    fn initialize(&self) -> LifecycleFuture<'_> {
        Box::pin(async move {
            self.ensure_chain_cache_initialized()
                .map_err(|error| Box::new(error) as vernal_core::BoxError)?;
            self.initialize_rule()
                .map_err(|error| Box::new(error) as vernal_core::BoxError)?;
            self.start_monitor_task();
            Ok(())
        })
    }

    fn stop(&self) -> LifecycleFuture<'_> {
        let monitor_task = self
            .monitor_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        Box::pin(async move {
            if let Some(monitor_task) = monitor_task {
                monitor_task.abort();
                // JoinError::is_cancelled 是正常关闭结果；任务内部无错误返回通道。
                let _ = monitor_task.await;
            }
            Ok(())
        })
    }
}
