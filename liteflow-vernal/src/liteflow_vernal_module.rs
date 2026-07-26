//! 对应 Java: `LiteflowMainAutoConfiguration` 与 `ComponentScanner`。

use std::sync::Arc;

use liteflow_core::log::LFLoggerManager;
use liteflow_core::{ExecutorHelper, FlowBus};
use vernal_beans::ComponentDefinition;
use vernal_context::{ApplicationModule, ApplicationModuleRegistrar};
use vernal_core::BoxError;

use crate::{
    LiteflowComponentRegistration, LiteflowConfig, LiteflowConfigGetter, LiteflowRuntime,
    LiteflowVernalError,
};

/// 原子贡献 FlowBus、LiteflowRuntime、生命周期与组件注册的 Vernal 模块。
pub struct LiteflowVernalModule {
    config: LiteflowConfig,
    registrations: Vec<LiteflowComponentRegistration>,
}

impl LiteflowVernalModule {
    /// 使用类型安全配置创建模块。
    #[must_use]
    pub fn new(config: LiteflowConfig) -> Self {
        Self {
            config,
            registrations: Vec::new(),
        }
    }

    /// 追加一个组件注册动作。
    #[must_use]
    pub fn with_component(mut self, registration: LiteflowComponentRegistration) -> Self {
        self.registrations.push(registration);
        self
    }

    /// 批量追加组件注册动作。
    #[must_use]
    pub fn with_components(
        mut self,
        registrations: impl IntoIterator<Item = LiteflowComponentRegistration>,
    ) -> Self {
        self.registrations.extend(registrations);
        self
    }
}

impl ApplicationModule for LiteflowVernalModule {
    fn name(&self) -> &'static str {
        "liteflow.vernal"
    }

    fn configure(self, registrar: &mut ApplicationModuleRegistrar) -> Result<(), BoxError> {
        // Java LiteflowConfig 的线程参数在 Vernal 装配边界写入核心执行器注册中心，
        // 之后 WHEN、异步循环与 execute2Future 共享同一冻结配置。
        ExecutorHelper::load_instance().configure(
            self.config.global_thread_pool_executor_class.clone(),
            self.config.main_executor_class.clone(),
            self.config.global_thread_pool_size,
            self.config.global_thread_pool_queue_size,
            self.config.main_executor_works,
            self.config.when_thread_pool_isolate,
            self.config.enable_virtual_thread,
        );
        LFLoggerManager::set_print_execution_log(self.config.print_execution_log);
        // 与 Java FlowExecutor(LiteflowConfig) 一致，在运行时创建前登记兼容配置。
        LiteflowConfigGetter::set_liteflow_config(self.config.clone());
        let flow_bus = FlowBus::new();
        for registration in &self.registrations {
            registration.apply(&flow_bus).map_err(|error| {
                Box::new(LiteflowVernalError::ComponentRegistration {
                    component_id: registration.component_id().to_string(),
                    message: error.to_string(),
                }) as BoxError
            })?;
        }

        let runtime = Arc::new(LiteflowRuntime::new(flow_bus.clone(), self.config));
        registrar
            .register(ComponentDefinition::shared_value(flow_bus))
            .register(ComponentDefinition::shared_arc(runtime))
            .lifecycle::<LiteflowRuntime>();
        Ok(())
    }
}
