//! 对应 Java: `ComponentScanner` 对普通组件和声明式组件的注册结果。

use std::sync::Arc;

use liteflow_core::aop::ICmpAroundAspect;
use liteflow_core::core::proxy::{DeclWarpBean, LiteFlowProxyUtil};
use liteflow_core::lifecycle::LifeCycle;
use liteflow_core::script::ScriptBeanManager;
use liteflow_core::script::proxy::ScriptBeanProxy;
use liteflow_core::{FlowBus, LFResult, NodeComponent};

use crate::SharedRegistration;

/// 一个显式、可组合的 LiteFlow 组件注册动作。
///
/// Vernal 明确不做 classpath 扫描，因此 Rust 侧在应用模块构建期提交注册动作；
/// `liteflow-derive` 生成的组件注册方法可以直接包装到本对象中。
#[derive(Clone)]
pub struct LiteflowComponentRegistration {
    component_id: String,
    registration: SharedRegistration,
    managed_component: Option<Arc<dyn NodeComponent>>,
    decl_warp_bean: Option<DeclWarpBean>,
    cmp_around_aspect: Option<Arc<dyn ICmpAroundAspect>>,
    script_bean_proxies: Option<Vec<ScriptBeanProxy>>,
    script_method_proxies: Option<Vec<ScriptBeanProxy>>,
    life_cycle: Option<Arc<dyn LifeCycle>>,
    refresh_scoped: bool,
    bean_type_name: String,
}

impl LiteflowComponentRegistration {
    /// 创建组件注册动作。
    pub fn new<F>(component_id: impl Into<String>, registration: F) -> Self
    where
        F: Fn(&FlowBus) -> LFResult<()> + Send + Sync + 'static,
    {
        Self {
            component_id: component_id.into(),
            registration: std::sync::Arc::new(registration),
            managed_component: None,
            decl_warp_bean: None,
            cmp_around_aspect: None,
            script_bean_proxies: None,
            script_method_proxies: None,
            life_cycle: None,
            refresh_scoped: false,
            bean_type_name: "custom registration".to_string(),
        }
    }

    /// 创建一个由 Vernal 容器托管的普通节点注册动作。
    ///
    /// # 参数
    /// - `component_id`：EL 中引用的节点 ID；
    /// - `node_component`：Vernal 与 LiteFlow 共享的真实组件实例。
    ///
    /// # 返回
    /// 将由 `VernalContextCmpInit` 在规则解析前提交的注册动作。对应 Java:
    /// `NodeCmpBeanProcess` 与 `SpringContextCmpInit#initCmp`。
    pub fn managed(
        component_id: impl Into<String>,
        node_component: Arc<dyn NodeComponent>,
    ) -> Self {
        let component_id = component_id.into();
        let registered_component = Arc::clone(&node_component);
        let registered_id = component_id.clone();
        Self {
            component_id,
            registration: Arc::new(move |flow_bus| {
                flow_bus.add_managed_node(registered_id.clone(), Arc::clone(&registered_component))
            }),
            managed_component: Some(node_component),
            decl_warp_bean: None,
            cmp_around_aspect: None,
            script_bean_proxies: None,
            script_method_proxies: None,
            life_cycle: None,
            refresh_scoped: false,
            bean_type_name: "dyn liteflow_core::NodeComponent".to_string(),
        }
    }

    /// 创建一个由 Vernal 容器托管的声明式组件注册动作。
    ///
    /// `liteflow-derive` 已把 Java 运行期反射生成的注解信息写入
    /// `DeclWarpBean`；该入口把同一包装对象同时交给 Vernal 命名容器和
    /// `VernalDeclComponentParser` 驱动的 FlowBus 注册链。
    ///
    /// # 参数
    /// - `decl_warp_bean`：包含节点身份、方法元数据和原始单例对象的包装对象。
    ///
    /// # 返回
    /// 可追加到 `LiteflowVernalModule` 的声明式注册动作。对应 Java:
    /// `DeclBeanDefinition#splitAndRegisterNewBeanDefinition`。
    #[must_use]
    pub fn declarative(decl_warp_bean: DeclWarpBean) -> Self {
        let component_id = decl_warp_bean.node_id().to_string();
        let registered_decl_warp_bean = decl_warp_bean.clone();
        Self {
            component_id,
            registration: Arc::new(move |flow_bus| {
                flow_bus.try_register_decl_warp(registered_decl_warp_bean.clone())
            }),
            managed_component: None,
            decl_warp_bean: Some(decl_warp_bean),
            cmp_around_aspect: None,
            script_bean_proxies: None,
            script_method_proxies: None,
            life_cycle: None,
            refresh_scoped: false,
            bean_type_name: "liteflow_core::DeclWarpBean".to_string(),
        }
    }

    /// 创建已经完成声明定义拆分和校验的注册动作。
    ///
    /// 该入口只供 `VernalDeclBeanDefinition` 使用，避免已经由
    /// `DeclComponentParserHolder` 解析过的包装对象在组件扫描阶段被重复解析。
    ///
    /// # 参数
    /// - `decl_warp_bean`：已经按节点 ID 拆分并通过声明式解析器校验的包装对象。
    ///
    /// # 返回
    /// 直接生成代理并注册到 `FlowBus` 的内部注册动作。对应 Java:
    /// `DeclBeanDefinition#registerNewBeanDefinition` 之后的
    /// `DeclWarpBeanProcess#postProcessAfterInitialization`。
    pub(crate) fn parsed_declarative(decl_warp_bean: DeclWarpBean) -> Self {
        let component_id = decl_warp_bean.node_id().to_string();
        let registered_decl_warp_bean = decl_warp_bean.clone();
        Self {
            component_id,
            registration: Arc::new(move |flow_bus| {
                LiteFlowProxyUtil::register_decl_warp(flow_bus, registered_decl_warp_bean.clone())
            }),
            managed_component: None,
            decl_warp_bean: Some(decl_warp_bean),
            cmp_around_aspect: None,
            script_bean_proxies: None,
            script_method_proxies: None,
            life_cycle: None,
            refresh_scoped: false,
            bean_type_name: "liteflow_core::DeclWarpBean".to_string(),
        }
    }

    /// 创建组件切面 Bean 注册定义。
    ///
    /// # 参数
    /// - `bean_name`：Vernal 容器中的切面 Bean 名称；
    /// - `cmp_around_aspect`：真实业务切面共享实例。
    ///
    /// # 返回
    /// 可由 `CmpAroundAspectBeanProcess` 识别的注册定义；脱离扫描器直接
    /// `apply` 时也会把同一切面绑定到给定 `FlowBus`。对应 Java:
    /// `CmpAroundAspectBeanProcess#postProcessAfterInitialization`。
    #[must_use]
    pub fn cmp_around_aspect(
        bean_name: impl Into<String>,
        cmp_around_aspect: Arc<dyn ICmpAroundAspect>,
    ) -> Self {
        let registered_aspect = Arc::clone(&cmp_around_aspect);
        Self {
            component_id: bean_name.into(),
            registration: Arc::new(move |flow_bus| {
                flow_bus.register_aspect(Arc::clone(&registered_aspect));
                Ok(())
            }),
            managed_component: None,
            decl_warp_bean: None,
            cmp_around_aspect: Some(cmp_around_aspect),
            script_bean_proxies: None,
            script_method_proxies: None,
            life_cycle: None,
            refresh_scoped: false,
            bean_type_name: "dyn liteflow_core::ICmpAroundAspect".to_string(),
        }
    }

    /// 创建 `@ScriptBean` 等价注册定义。
    ///
    /// # 参数
    /// - `bean_name`：原始容器 Bean 名称；
    /// - `script_bean_proxy`：已应用 include/exclude 白名单的真实脚本代理。
    ///
    /// # 返回
    /// 可由 `ScriptBeanProcess` 识别并写入 `ScriptBeanManager` 的定义。
    #[must_use]
    pub fn script_bean(bean_name: impl Into<String>, script_bean_proxy: ScriptBeanProxy) -> Self {
        let registered_proxy = script_bean_proxy.clone();
        Self {
            component_id: bean_name.into(),
            registration: Arc::new(move |_| {
                ScriptBeanManager::add_script_bean(registered_proxy.clone());
                Ok(())
            }),
            managed_component: None,
            decl_warp_bean: None,
            cmp_around_aspect: None,
            script_bean_proxies: Some(vec![script_bean_proxy]),
            script_method_proxies: None,
            life_cycle: None,
            refresh_scoped: false,
            bean_type_name: "liteflow_core::ScriptBean".to_string(),
        }
    }

    /// 创建 `@ScriptMethod` 分组后的注册定义。
    ///
    /// # 参数
    /// - `bean_name`：声明脚本方法的原始容器 Bean 名称；
    /// - `script_method_proxies`：按注解 value 分组构造的一个或多个脚本 Bean 代理。
    ///
    /// # 返回
    /// 可由 `ScriptMethodBeanProcess` 逐组注册的定义。对应 Java:
    /// `ScriptMethodBeanProcess#postProcessAfterInitialization`。
    #[must_use]
    pub fn script_methods(
        bean_name: impl Into<String>,
        script_method_proxies: Vec<ScriptBeanProxy>,
    ) -> Self {
        let registered_proxies = script_method_proxies.clone();
        Self {
            component_id: bean_name.into(),
            registration: Arc::new(move |_| {
                for proxy in &registered_proxies {
                    ScriptBeanManager::add_script_bean(proxy.clone());
                }
                Ok(())
            }),
            managed_component: None,
            decl_warp_bean: None,
            cmp_around_aspect: None,
            script_bean_proxies: None,
            script_method_proxies: Some(script_method_proxies),
            life_cycle: None,
            refresh_scoped: false,
            bean_type_name: "liteflow_core::ScriptMethod".to_string(),
        }
    }

    /// 创建生命周期 Bean 注册定义。
    ///
    /// # 参数
    /// - `bean_name`：生命周期对象的容器名称；
    /// - `life_cycle`：可动态分派到真实生命周期阶段的共享对象。
    ///
    /// # 返回
    /// 可由 `LifeCycleBeanProcess` 写入当前 `FlowBus` 隔离持有器的定义。
    #[must_use]
    pub fn life_cycle(bean_name: impl Into<String>, life_cycle: Arc<dyn LifeCycle>) -> Self {
        let registered_life_cycle = Arc::clone(&life_cycle);
        Self {
            component_id: bean_name.into(),
            registration: Arc::new(move |flow_bus| {
                flow_bus.register_life_cycle(Arc::clone(&registered_life_cycle));
                Ok(())
            }),
            managed_component: None,
            decl_warp_bean: None,
            cmp_around_aspect: None,
            script_bean_proxies: None,
            script_method_proxies: None,
            life_cycle: Some(life_cycle),
            refresh_scoped: false,
            bean_type_name: "dyn liteflow_core::LifeCycle".to_string(),
        }
    }

    /// 标记该定义对应 Spring Cloud `@RefreshScope` 代理目标。
    ///
    /// # 返回
    /// 扫描普通节点时会去除 `scopedTarget.` 前缀的新定义。Rust 没有运行期
    /// Java 注解反射，因此由显式强类型模块携带同一事实。
    #[must_use]
    pub fn with_refresh_scope(mut self) -> Self {
        self.refresh_scoped = true;
        self
    }

    /// 返回组件 id，用于结构化诊断。
    #[must_use]
    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    /// 向给定 FlowBus 提交注册。
    pub fn apply(&self, flow_bus: &FlowBus) -> LFResult<()> {
        (self.registration)(flow_bus)
    }

    /// 返回 Vernal 托管的节点实例。
    ///
    /// # 返回
    /// `managed` 构造的注册返回同一共享实例；普通自定义注册返回 `None`。
    #[must_use]
    pub fn managed_component(&self) -> Option<Arc<dyn NodeComponent>> {
        self.managed_component.clone()
    }

    /// 返回声明式组件包装对象。
    ///
    /// # 返回
    /// `declarative` 构造的注册返回同一份共享业务对象元数据；其他注册返回
    /// `None`。对应 Java: `DeclBeanDefinition#registerNewBeanDefinition`。
    #[must_use]
    pub fn decl_warp_bean(&self) -> Option<DeclWarpBean> {
        self.decl_warp_bean.clone()
    }

    /// 返回组件切面实例。
    ///
    /// # 返回
    /// 切面注册定义中的同一共享对象，其他定义返回 `None`。
    #[must_use]
    pub fn cmp_around_aspect_instance(&self) -> Option<Arc<dyn ICmpAroundAspect>> {
        self.cmp_around_aspect.clone()
    }

    /// 返回 `@ScriptBean` 解析出的代理列表。
    ///
    /// # 返回
    /// 脚本 Bean 定义返回非空列表；其他定义返回 `None`。
    #[must_use]
    pub fn script_bean_proxies(&self) -> Option<Vec<ScriptBeanProxy>> {
        self.script_bean_proxies.clone()
    }

    /// 返回 `@ScriptMethod` 按 value 分组后的代理列表。
    ///
    /// # 返回
    /// 脚本方法定义返回分组代理快照；其他定义返回 `None`。
    #[must_use]
    pub fn script_method_proxies(&self) -> Option<Vec<ScriptBeanProxy>> {
        self.script_method_proxies.clone()
    }

    /// 返回生命周期扩展对象。
    ///
    /// # 返回
    /// 生命周期定义中的共享对象，其他定义返回 `None`。
    #[must_use]
    pub fn life_cycle_instance(&self) -> Option<Arc<dyn LifeCycle>> {
        self.life_cycle.clone()
    }

    /// 返回定义是否来自 `@RefreshScope` 代理目标。
    #[must_use]
    pub fn is_refresh_scoped(&self) -> bool {
        self.refresh_scoped
    }

    /// 返回原始 Bean 的类型诊断名称。
    ///
    /// Rust 扫描步骤依赖强类型字段判断；该名称保留 Java Context 中 `clazz`
    /// 的日志和诊断用途。
    #[must_use]
    pub fn bean_type_name(&self) -> &str {
        &self.bean_type_name
    }
}
