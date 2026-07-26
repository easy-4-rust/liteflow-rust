//! 对应 Java: `FlowExecutor` + `LiteflowExecutorInit` 的 Vernal 托管对象。

use std::sync::Mutex;
use std::time::Duration;

use liteflow_core::monitor::MonitorTimeTask;
use liteflow_core::{FlowBus, LiteflowResponse, rule};
use serde_json::Value;
use vernal_context::{Lifecycle, LifecycleFuture};

use crate::{LiteflowConfig, LiteflowRuleFormat, LiteflowVernalError};

/// 由 Vernal 管理生命周期的 LiteFlow 运行时。
pub struct LiteflowRuntime {
    flow_bus: FlowBus,
    config: LiteflowConfig,
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
