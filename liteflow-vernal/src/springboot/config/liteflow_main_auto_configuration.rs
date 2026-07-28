use std::sync::Arc;

use liteflow_core::aop::ICmpAroundAspect;
use vernal_context::{ApplicationModule, ApplicationModuleRegistrar};
use vernal_core::BoxError;

use crate::{LiteflowComponentRegistration, LiteflowVernalConfig, LiteflowVernalModule};

/// LiteFlow 的 Vernal/Axum 主自动配置对象。
///
/// Java 依据 `liteflow.enable=true` 创建执行器、扫描器、监控器和 SPI 初始化对象；
/// Rust 保留相同条件，并把具体 Bean 装配交给 `LiteflowVernalModule`。对应 Java:
/// `com.yomahub.liteflow.springboot.config.LiteflowMainAutoConfiguration`。
pub struct LiteflowMainAutoConfiguration {
    config: LiteflowVernalConfig,
    registrations: Vec<LiteflowComponentRegistration>,
    cmp_around_aspect: Option<Arc<dyn ICmpAroundAspect>>,
}

impl LiteflowMainAutoConfiguration {
    /// 使用统一配置创建主自动配置。
    #[must_use]
    pub fn new(config: LiteflowVernalConfig) -> Self {
        Self {
            config,
            registrations: Vec::new(),
            cmp_around_aspect: None,
        }
    }

    /// 追加一个容器组件定义。
    #[must_use]
    pub fn with_component(mut self, registration: LiteflowComponentRegistration) -> Self {
        self.registrations.push(registration);
        self
    }

    /// 批量追加容器组件定义。
    #[must_use]
    pub fn with_components(
        mut self,
        registrations: impl IntoIterator<Item = LiteflowComponentRegistration>,
    ) -> Self {
        self.registrations.extend(registrations);
        self
    }

    /// 设置全局组件切面。
    #[must_use]
    pub fn with_cmp_around_aspect(mut self, cmp_around_aspect: Arc<dyn ICmpAroundAspect>) -> Self {
        self.cmp_around_aspect = Some(cmp_around_aspect);
        self
    }

    /// 返回是否满足 Java 自动配置启用条件。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enable
    }
}

impl ApplicationModule for LiteflowMainAutoConfiguration {
    fn name(&self) -> &'static str {
        "liteflow.springboot.main-auto-configuration"
    }

    fn configure(self, registrar: &mut ApplicationModuleRegistrar) -> Result<(), BoxError> {
        if !self.config.enable {
            // 对应 @ConditionalOnProperty(liteflow.enable=true)：关闭时不贡献任何 Bean。
            return Ok(());
        }
        let mut module = LiteflowVernalModule::new(self.config).with_components(self.registrations);
        if let Some(cmp_around_aspect) = self.cmp_around_aspect {
            module = module.with_cmp_around_aspect(cmp_around_aspect);
        }
        ApplicationModule::configure(module, registrar)
    }
}
