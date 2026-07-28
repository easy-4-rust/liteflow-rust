use std::sync::Arc;

use liteflow_core::aop::ICmpAroundAspect;
use vernal_context::{ApplicationModule, ApplicationModuleRegistrar};
use vernal_core::BoxError;

use crate::{LiteflowComponentRegistration, LiteflowVernalConfig, LiteflowVernalModule};

/// LiteFlow 的 Spring Boot 4/Vernal 主自动配置对象。
///
/// Java Boot 4 以 `@AutoConfiguration` 参与新自动配置发现机制；Rust 将该边界
/// 表达为独立 `ApplicationModule`，并保留 `liteflow.enable=true` 条件。对应
/// Java: `com.yomahub.liteflow.springboot4.config.LiteflowMainAutoConfiguration`。
pub struct LiteflowMainAutoConfiguration {
    config: LiteflowVernalConfig,
    registrations: Vec<LiteflowComponentRegistration>,
    cmp_around_aspect: Option<Arc<dyn ICmpAroundAspect>>,
}

impl LiteflowMainAutoConfiguration {
    /// 使用 Boot 4 统一配置创建主自动配置。
    #[must_use]
    pub fn new(config: LiteflowVernalConfig) -> Self {
        Self {
            config,
            registrations: Vec::new(),
            cmp_around_aspect: None,
        }
    }

    /// 追加一个容器组件定义。
    ///
    /// # 参数
    /// - `registration`：待加入真实 FlowBus/Vernal 容器的组件注册。
    #[must_use]
    pub fn with_component(mut self, registration: LiteflowComponentRegistration) -> Self {
        self.registrations.push(registration);
        self
    }

    /// 批量追加容器组件定义。
    ///
    /// # 参数
    /// - `registrations`：组件注册迭代器。
    #[must_use]
    pub fn with_components(
        mut self,
        registrations: impl IntoIterator<Item = LiteflowComponentRegistration>,
    ) -> Self {
        self.registrations.extend(registrations);
        self
    }

    /// 设置全局组件切面。
    ///
    /// # 参数
    /// - `cmp_around_aspect`：Vernal 托管的 LiteFlow 切面。
    #[must_use]
    pub fn with_cmp_around_aspect(mut self, cmp_around_aspect: Arc<dyn ICmpAroundAspect>) -> Self {
        self.cmp_around_aspect = Some(cmp_around_aspect);
        self
    }

    /// 返回是否满足 Java Boot 4 自动配置启用条件。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enable
    }
}

impl ApplicationModule for LiteflowMainAutoConfiguration {
    fn name(&self) -> &'static str {
        "liteflow.springboot4.main-auto-configuration"
    }

    fn configure(self, registrar: &mut ApplicationModuleRegistrar) -> Result<(), BoxError> {
        if !self.config.enable {
            // 对应 @ConditionalOnProperty(liteflow.enable=true)：关闭时不贡献 Bean。
            return Ok(());
        }
        let mut module = LiteflowVernalModule::new(self.config)
            .with_components(self.registrations)
            .with_springboot4_executor_init();
        if let Some(cmp_around_aspect) = self.cmp_around_aspect {
            module = module.with_cmp_around_aspect(cmp_around_aspect);
        }
        ApplicationModule::configure(module, registrar)
    }
}
