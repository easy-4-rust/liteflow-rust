//! LiteFlow 的 Rust/Vernal 原子装配门面。
//!
//! 本类型负责复用具体 Bean 接线，不额外对应 Java 对象；Java
//! `LiteflowMainAutoConfiguration` 已独立迁入
//! `springboot/config/liteflow_main_auto_configuration.rs`。

use std::sync::Arc;

use liteflow_core::aop::ICmpAroundAspect;
use liteflow_core::log::LFLoggerManager;
use liteflow_core::spi::{
    CmpAroundAspectHolder, ContextAwareHolder, ContextCmpInitHolder, DeclComponentParserHolder,
    LiteflowComponentSupportHolder, PathContentParserHolder,
};
use liteflow_core::{ExecutorHelper, FlowBus};
use vernal_beans::ComponentDefinition;
use vernal_context::{ApplicationModule, ApplicationModuleRegistrar};
use vernal_core::BoxError;

use crate::process::holder::SolonNodeIdHolder;
use crate::solon::integration::XPluginImpl;
use crate::spi::solon::{
    SolonCmpAroundAspect, SolonContextAware, SolonContextCmpInit, SolonDeclComponentParser,
    SolonLiteflowComponentSupport, SolonPathContentParser,
};
use crate::{
    LiteflowComponentRegistration, LiteflowConfig, LiteflowConfigGetter, LiteflowExecutorInit,
    LiteflowRuntime, LiteflowSpiInit, LiteflowVernalError, VernalAware, VernalCmpAroundAspect,
    VernalComponentScanner, VernalContextCmpInit, VernalDeclBeanDefinition,
    VernalDeclComponentParser, VernalLiteflowComponentSupport, VernalPathContentParser,
};

/// 原子贡献 FlowBus、LiteflowRuntime、生命周期与组件注册的 Rust 组合模块。
pub struct LiteflowVernalModule {
    config: LiteflowConfig,
    registrations: Vec<LiteflowComponentRegistration>,
    cmp_around_aspect: Option<Arc<dyn ICmpAroundAspect>>,
    springboot4_executor_init: bool,
    parse_on_start: bool,
    register_executor_init: bool,
    solon_spi: bool,
}

impl LiteflowVernalModule {
    /// 使用类型安全配置创建模块。
    #[must_use]
    pub fn new(config: LiteflowConfig) -> Self {
        Self {
            config,
            registrations: Vec::new(),
            cmp_around_aspect: None,
            springboot4_executor_init: false,
            parse_on_start: true,
            register_executor_init: true,
            solon_spi: false,
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

    /// 配置一个由 Vernal 托管的全局组件切面。
    ///
    /// # 参数
    /// - `cmp_around_aspect`：实现 Java `ICmpAroundAspect` 语义的共享对象。
    ///
    /// # 返回
    /// 包含切面注册的模块构建器。对应 Java:
    /// `CmpAroundAspectBeanProcess#postProcessAfterInitialization`。
    #[must_use]
    pub fn with_cmp_around_aspect(mut self, cmp_around_aspect: Arc<dyn ICmpAroundAspect>) -> Self {
        self.cmp_around_aspect = Some(cmp_around_aspect);
        self
    }

    /// 使用 Spring Boot 4 对应的独立执行器初始化对象。
    ///
    /// # 返回
    /// 注册 `springboot4::LiteflowExecutorInit` 的组合模块。该入口仅由 Boot 4
    /// 自动配置调用，避免把 Boot 3 Java 对象错误注册进 Boot 4 容器。
    #[must_use]
    pub fn with_springboot4_executor_init(mut self) -> Self {
        self.springboot4_executor_init = true;
        self
    }

    /// 设置是否在容器启动阶段解析规则。
    ///
    /// # 参数
    /// - `parse_on_start`：`true` 表示启动期初始化，`false` 表示首次执行时初始化。
    ///
    /// 该入口由 Solon 主自动配置消费。对应 Java:
    /// `LiteflowMainAutoConfiguration#parseOnStart`。
    #[must_use]
    pub fn with_parse_on_start(mut self, parse_on_start: bool) -> Self {
        self.parse_on_start = parse_on_start;
        self
    }

    /// 不注册 Spring Boot 专属的 `LiteflowExecutorInit`。
    ///
    /// Solon 基线没有该 Java 对象，其 `@Init` 逻辑直接位于
    /// `LiteflowMainAutoConfiguration`；因此 Solon 模块只保留运行时生命周期。
    #[must_use]
    pub fn without_executor_init(mut self) -> Self {
        self.register_executor_init = false;
        self
    }

    /// 使用 Solon 专属 SPI 与插件启动链。
    ///
    /// # 返回
    /// 组合模块会注册六个 `Solon*` SPI、`XPluginImpl` 和 Solon 节点 Holder，
    /// 不再创建任何 Vernal/Spring SPI 对象。
    #[must_use]
    pub fn with_solon_spi(mut self) -> Self {
        self.solon_spi = true;
        self.register_executor_init = false;
        self
    }

    /// 配置 Solon 专属的容器启动链。
    fn configure_solon(self, registrar: &mut ApplicationModuleRegistrar) -> Result<(), BoxError> {
        let context_aware = Arc::new(SolonContextAware::new());
        let x_plugin = Arc::new(XPluginImpl::new(self.config.enable));

        if !self.config.enable {
            // Java 插件先加载默认属性，再在 liteflow.enable=false 时立即返回。
            x_plugin
                .start(
                    &context_aware,
                    &FlowBus::new(),
                    &[],
                    &SolonDeclComponentParser::new(),
                    &SolonNodeIdHolder::of(context_aware.as_ref()),
                )
                .map_err(|error| Box::new(error) as BoxError)?;
            registrar
                .register(ComponentDefinition::shared_arc(context_aware))
                .register(ComponentDefinition::shared_arc(x_plugin));
            return Ok(());
        }

        let decl_component_parser = Arc::new(SolonDeclComponentParser::new());
        let liteflow_component_support = Arc::new(SolonLiteflowComponentSupport::new());
        let path_content_parser = Arc::new(SolonPathContentParser::new());
        ContextAwareHolder::register(context_aware.clone());
        DeclComponentParserHolder::register(decl_component_parser.clone());
        LiteflowComponentSupportHolder::register(liteflow_component_support.clone());
        PathContentParserHolder::register(path_content_parser.clone());

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
        LiteflowConfigGetter::set_liteflow_config(self.config.to_core_config());
        context_aware.register_typed_bean("liteflowConfig", Arc::new(self.config.clone()));

        let flow_bus = Arc::new(FlowBus::new());
        context_aware.register_typed_bean("flowBus", Arc::clone(&flow_bus));
        let node_id_holder = SolonNodeIdHolder::of(context_aware.as_ref());

        let mut registrations = self.registrations;
        if let Some(cmp_around_aspect) = self.cmp_around_aspect {
            registrations.push(LiteflowComponentRegistration::cmp_around_aspect(
                "cmpAroundAspect",
                cmp_around_aspect,
            ));
        }
        let business_aspect = registrations
            .iter()
            .find_map(LiteflowComponentRegistration::cmp_around_aspect_instance);
        let cmp_around_aspect = Arc::new(SolonCmpAroundAspect::new(business_aspect));
        CmpAroundAspectHolder::register(cmp_around_aspect.clone());
        if cmp_around_aspect.get_cmp_around_aspect().is_some() {
            flow_bus.register_aspect(cmp_around_aspect.clone());
        }

        // XPluginImpl 先处理默认配置、声明组件、生命周期和普通注册定义，再由
        // ContextCmpInit 统一提交 Holder 中的容器节点。
        x_plugin
            .start(
                &context_aware,
                &flow_bus,
                &registrations,
                &decl_component_parser,
                &node_id_holder,
            )
            .map_err(|error| Box::new(error) as BoxError)?;
        let context_cmp_init = Arc::new(SolonContextCmpInit::new(
            (*flow_bus).clone(),
            Arc::clone(&context_aware),
            Arc::clone(&node_id_holder),
        ));
        ContextCmpInitHolder::register(context_cmp_init.clone());
        let liteflow_spi_init = Arc::new(LiteflowSpiInit::new());
        liteflow_spi_init.after_singletons_instantiated();
        context_cmp_init.try_init_cmp().map_err(|error| {
            Box::new(LiteflowVernalError::ManagedComponentInitialization(
                error.to_string(),
            )) as BoxError
        })?;

        let runtime = Arc::new(LiteflowRuntime::with_initialize_on_start(
            (*flow_bus).clone(),
            self.config,
            self.parse_on_start,
        ));
        context_aware.register_typed_bean("liteflowRuntime", Arc::clone(&runtime));
        context_aware.register_typed_bean("liteflowSpiInit", Arc::clone(&liteflow_spi_init));
        context_aware.register_typed_bean("xPluginImpl", Arc::clone(&x_plugin));

        registrar
            .register(ComponentDefinition::shared_arc(flow_bus))
            .register(ComponentDefinition::shared_arc(Arc::clone(&runtime)))
            .register(ComponentDefinition::shared_arc(context_aware))
            .register(ComponentDefinition::shared_arc(cmp_around_aspect))
            .register(ComponentDefinition::shared_arc(context_cmp_init))
            .register(ComponentDefinition::shared_arc(decl_component_parser))
            .register(ComponentDefinition::shared_arc(liteflow_component_support))
            .register(ComponentDefinition::shared_arc(path_content_parser))
            .register(ComponentDefinition::shared_arc(node_id_holder))
            .register(ComponentDefinition::shared_arc(liteflow_spi_init))
            .register(ComponentDefinition::shared_arc(x_plugin))
            .lifecycle::<LiteflowRuntime>();
        Ok(())
    }
}

impl ApplicationModule for LiteflowVernalModule {
    fn name(&self) -> &'static str {
        "liteflow.vernal"
    }

    fn configure(self, registrar: &mut ApplicationModuleRegistrar) -> Result<(), BoxError> {
        if self.solon_spi {
            return self.configure_solon(registrar);
        }
        let context_aware = Arc::new(VernalAware::new());
        ContextAwareHolder::register(context_aware.clone());
        let decl_component_parser = Arc::new(VernalDeclComponentParser::new());
        let liteflow_component_support = Arc::new(VernalLiteflowComponentSupport::new());
        // Java 通过 ServiceLoader 选择优先级为 1 的 Spring 容器 SPI；Vernal
        // 显式装配相同对象，确保声明式组件和 @LiteflowComponent 名称进入真实
        // ComponentInitializer / FlowBus 运行链。
        DeclComponentParserHolder::register(decl_component_parser.clone());
        LiteflowComponentSupportHolder::register(liteflow_component_support.clone());
        // Spring 容器通过 ServiceLoader 选择优先级为 1 的 SpringPathContentParser；
        // Vernal 在模块装配边界显式注册同等实现，避免随后规则初始化退回本地解析器。
        PathContentParserHolder::register(Arc::new(VernalPathContentParser::new()));
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
        LiteflowConfigGetter::set_liteflow_config(self.config.to_core_config());
        context_aware.register_typed_bean("liteflowConfig", Arc::new(self.config.clone()));
        let flow_bus = Arc::new(FlowBus::new());
        context_aware.register_typed_bean("flowBus", Arc::clone(&flow_bus));
        let decl_bean_definition = Arc::new(VernalDeclBeanDefinition::new());
        let mut registrations = self.registrations;
        if let Some(cmp_around_aspect) = self.cmp_around_aspect {
            // 切面与普通 Bean 一样进入扫描步骤，避免模块构建器绕过 Java
            // CmpAroundAspectBeanProcess 的识别和 Holder 初始化语义。
            registrations.push(LiteflowComponentRegistration::cmp_around_aspect(
                "cmpAroundAspect",
                cmp_around_aspect,
            ));
        }
        let registrations = decl_bean_definition
            .post_process_bean_definition_registry(&registrations, &context_aware)
            .map_err(|error| {
                Box::new(LiteflowVernalError::ComponentRegistration {
                    component_id: "declBeanDefinition".to_string(),
                    message: error.to_string(),
                }) as BoxError
            })?;
        decl_bean_definition.post_process_bean_factory();
        let component_scanner = Arc::new(VernalComponentScanner::with_config(
            &self.config,
            registrations,
        ));
        let managed_registrations = component_scanner
            .scan(&flow_bus)
            .map_err(|error| Box::new(error) as BoxError)?;
        let spring_cmp_around_aspect_holder = component_scanner.spring_cmp_around_aspect_holder();
        let spring_node_id_holder = component_scanner.spring_node_id_holder();
        let process_step_factory = component_scanner.process_step_factory();
        let cmp_around_aspect = Arc::new(VernalCmpAroundAspect::new(
            spring_cmp_around_aspect_holder.get_instance(),
        ));
        CmpAroundAspectHolder::register(cmp_around_aspect.clone());
        if cmp_around_aspect.get_instance().is_some() {
            // FlowBus 保存同一 Vernal SPI 适配器；运行期四个回调继续委托给
            // SpringCmpAroundAspectHolder 扫描到的业务切面。
            flow_bus.register_aspect(cmp_around_aspect.clone());
        }
        let managed_nodes = managed_registrations
            .iter()
            .filter_map(|registration| {
                registration
                    .managed_component()
                    .map(|node_component| (registration.component_id().to_string(), node_component))
            })
            .collect();
        let context_cmp_init = Arc::new(VernalContextCmpInit::new(
            (*flow_bus).clone(),
            managed_nodes,
        ));
        ContextCmpInitHolder::register(context_cmp_init.clone());
        let liteflow_spi_init = Arc::new(LiteflowSpiInit::new());
        // Java SmartInitializingSingleton 在所有 SPI 单例准备完毕后统一预加载；
        // 此处严格晚于六个 Holder 注册，并早于规则和托管组件初始化。
        liteflow_spi_init.after_singletons_instantiated();
        context_cmp_init.try_init_cmp().map_err(|error| {
            Box::new(LiteflowVernalError::ManagedComponentInitialization(
                error.to_string(),
            )) as BoxError
        })?;

        let runtime = Arc::new(LiteflowRuntime::with_initialize_on_start(
            (*flow_bus).clone(),
            self.config,
            self.parse_on_start,
        ));
        context_aware.register_typed_bean("liteflowRuntime", Arc::clone(&runtime));
        context_aware.register_typed_bean("liteflowSpiInit", Arc::clone(&liteflow_spi_init));
        registrar
            .register(ComponentDefinition::shared_arc(flow_bus))
            .register(ComponentDefinition::shared_arc(Arc::clone(&runtime)))
            .register(ComponentDefinition::shared_arc(Arc::clone(&context_aware)))
            .register(ComponentDefinition::shared_arc(cmp_around_aspect))
            .register(ComponentDefinition::shared_arc(context_cmp_init))
            .register(ComponentDefinition::shared_arc(decl_component_parser))
            .register(ComponentDefinition::shared_arc(liteflow_component_support))
            .register(ComponentDefinition::shared_arc(decl_bean_definition))
            .register(ComponentDefinition::shared_arc(component_scanner))
            .register(ComponentDefinition::shared_arc(process_step_factory))
            .register(ComponentDefinition::shared_arc(spring_node_id_holder))
            .register(ComponentDefinition::shared_arc(
                spring_cmp_around_aspect_holder,
            ))
            .register(ComponentDefinition::shared_arc(liteflow_spi_init));
        if self.register_executor_init {
            // Solon 会关闭此分支，直接由 LiteflowRuntime 的 parse_on_start 控制；
            // Spring Boot 3/4 则继续注册各自独立的初始化对象。
            if self.springboot4_executor_init {
                let liteflow_executor_init = Arc::new(
                    crate::springboot4::LiteflowExecutorInit::new(Arc::clone(&runtime)),
                );
                context_aware.register_typed_bean(
                    "liteflowExecutorInit",
                    Arc::clone(&liteflow_executor_init),
                );
                registrar
                    .register(ComponentDefinition::shared_arc(liteflow_executor_init))
                    .lifecycle::<crate::springboot4::LiteflowExecutorInit>();
            } else {
                let liteflow_executor_init =
                    Arc::new(LiteflowExecutorInit::new(Arc::clone(&runtime)));
                context_aware.register_typed_bean(
                    "liteflowExecutorInit",
                    Arc::clone(&liteflow_executor_init),
                );
                registrar
                    .register(ComponentDefinition::shared_arc(liteflow_executor_init))
                    .lifecycle::<LiteflowExecutorInit>();
            }
        }
        registrar.lifecycle::<LiteflowRuntime>();
        Ok(())
    }
}
