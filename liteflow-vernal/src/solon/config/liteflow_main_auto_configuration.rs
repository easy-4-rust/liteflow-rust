use liteflow_core::aop::ICmpAroundAspect;
use std::sync::Arc;
use vernal_context::{ApplicationModule, ApplicationModuleRegistrar};
use vernal_core::BoxError;

use crate::{LiteflowComponentRegistration, LiteflowVernalConfig, LiteflowVernalModule};

/// Solon 环境的 LiteFlow 主业务装配器。
///
/// Java 在 `@Init` 阶段创建 `FlowExecutor`，按 `parseOnStart` 决定是否立即初始化，
/// 再放入 `AppContext`；Rust 将同一职责映射为 Vernal `ApplicationModule`，注册
/// `LiteflowRuntime` 并由容器生命周期驱动。对应 Java:
/// `com.yomahub.liteflow.solon.config.LiteflowMainAutoConfiguration`。
pub struct LiteflowMainAutoConfiguration {
    parse_on_start: bool,
    liteflow_config: LiteflowVernalConfig,
    registrations: Vec<LiteflowComponentRegistration>,
    cmp_around_aspect: Option<Arc<dyn ICmpAroundAspect>>,
}

impl LiteflowMainAutoConfiguration {
    /// 使用统一配置创建主自动配置，默认启动期解析规则。
    ///
    /// # 参数
    /// - `liteflow_config`：`LiteflowAutoConfiguration#liteflowConfig` 的合并结果。
    #[must_use]
    pub fn new(liteflow_config: LiteflowVernalConfig) -> Self {
        Self {
            parse_on_start: true,
            liteflow_config,
            registrations: Vec::new(),
            cmp_around_aspect: None,
        }
    }

    /// 设置是否在容器启动期解析规则。
    ///
    /// # 参数
    /// - `parse_on_start`：对应 Solon 属性 `liteflow.parseOnStart`。
    #[must_use]
    pub fn with_parse_on_start(mut self, parse_on_start: bool) -> Self {
        self.parse_on_start = parse_on_start;
        self
    }

    /// 追加一个 Solon/Vernal 托管组件。
    ///
    /// # 参数
    /// - `registration`：组件 ID 与真实节点对象的注册描述。
    #[must_use]
    pub fn with_component(mut self, registration: LiteflowComponentRegistration) -> Self {
        self.registrations.push(registration);
        self
    }

    /// 批量追加 Solon/Vernal 托管组件。
    ///
    /// # 参数
    /// - `registrations`：待注册组件集合。
    #[must_use]
    pub fn with_components(
        mut self,
        registrations: impl IntoIterator<Item = LiteflowComponentRegistration>,
    ) -> Self {
        self.registrations.extend(registrations);
        self
    }

    /// 设置全局组件执行切面。
    ///
    /// # 参数
    /// - `cmp_around_aspect`：与 Java `ICmpAroundAspect` 对等的共享切面。
    #[must_use]
    pub fn with_cmp_around_aspect(mut self, cmp_around_aspect: Arc<dyn ICmpAroundAspect>) -> Self {
        self.cmp_around_aspect = Some(cmp_around_aspect);
        self
    }

    /// 返回是否在启动阶段解析规则。
    ///
    /// # 返回
    /// `true` 表示对等执行 Java `flowExecutor.init(true)`。
    #[must_use]
    pub fn is_parse_on_start(&self) -> bool {
        self.parse_on_start
    }

    /// 执行主业务装配。
    ///
    /// Vernal 的 `configure` 阶段对等 Java `@Init flowExecutor`：创建运行时、注入
    /// 配置并注册到容器；`parse_on_start` 决定后续生命周期是否立即解析规则。
    /// 对应 Java: `LiteflowMainAutoConfiguration#flowExecutor`。
    pub fn flow_executor(self, registrar: &mut ApplicationModuleRegistrar) -> Result<(), BoxError> {
        let mut module = LiteflowVernalModule::new(self.liteflow_config)
            .with_components(self.registrations)
            .with_parse_on_start(self.parse_on_start)
            .with_solon_spi();
        if let Some(cmp_around_aspect) = self.cmp_around_aspect {
            module = module.with_cmp_around_aspect(cmp_around_aspect);
        }
        ApplicationModule::configure(module, registrar)
    }
}

impl ApplicationModule for LiteflowMainAutoConfiguration {
    fn name(&self) -> &'static str {
        "liteflow.solon.main-auto-configuration"
    }

    fn configure(self, registrar: &mut ApplicationModuleRegistrar) -> Result<(), BoxError> {
        self.flow_executor(registrar)
    }
}
