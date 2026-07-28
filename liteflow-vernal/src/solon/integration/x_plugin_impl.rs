use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use liteflow_core::core::proxy::LiteFlowProxyUtil;
use liteflow_core::{FlowBus, LFResult};

use crate::LiteflowComponentRegistration;
use crate::process::holder::SolonNodeIdHolder;
use crate::solon::config::{LiteflowAutoConfiguration, LiteflowMonitorProperty, LiteflowProperty};
use crate::spi::solon::{SolonContextAware, SolonDeclComponentParser};

/// LiteFlow 的 Solon 插件启动实现。
///
/// Java `Plugin#start` 依次加载默认配置、判断启用开关、提前创建四个配置 Bean、
/// 订阅生命周期与普通节点、提取 `@LiteflowMethod` 声明组件，并处理
/// `@LiteflowComponent` 节点身份。Rust 没有运行期注解反射，相关元数据由
/// `liteflow-derive` 和 `LiteflowComponentRegistration` 在编译期提交；本对象
/// 执行同样的启动排序和真实 FlowBus/上下文注册。对应 Java:
/// `com.yomahub.liteflow.solon.integration.XPluginImpl`。
pub struct XPluginImpl {
    enable: bool,
    default_properties_loaded: AtomicBool,
    started: AtomicBool,
}

impl XPluginImpl {
    /// 使用 `liteflow.enable` 创建插件。
    ///
    /// # 参数
    /// - `enable`：Solon 配置中的启用开关。
    #[must_use]
    pub fn new(enable: bool) -> Self {
        Self {
            enable,
            default_properties_loaded: AtomicBool::new(false),
            started: AtomicBool::new(false),
        }
    }

    /// 启动 Solon 插件并处理全部已提交组件。
    ///
    /// # 参数
    /// - `context`：当前 Solon 应用上下文；
    /// - `flow_bus`：当前 LiteFlow 注册总线；
    /// - `registrations`：编译期/应用模块提交的组件定义；
    /// - `decl_component_parser`：Solon 声明式组件解析 SPI；
    /// - `node_id_holder`：当前上下文的节点 ID 附件。
    ///
    /// # 返回
    /// 全部声明式组件、生命周期和普通注册动作提交成功时返回 `Ok(())`。
    /// 对应 Java: `XPluginImpl#start(AppContext)`。
    pub fn start(
        &self,
        context: &Arc<SolonContextAware>,
        flow_bus: &FlowBus,
        registrations: &[LiteflowComponentRegistration],
        decl_component_parser: &SolonDeclComponentParser,
        node_id_holder: &Arc<SolonNodeIdHolder>,
    ) -> LFResult<()> {
        // Java 首先加载 META-INF/liteflow-default.properties；Rust 默认值编译进
        // serde 属性对象，先标记配置源已准备，再判断 enable。
        self.default_properties_loaded
            .store(true, Ordering::Release);

        if !self.enable {
            return Ok(());
        }

        // Java 只在启用后 beanMake 四个配置对象；Rust 主自动配置本身正处于
        // configure 调用中，插件在上下文中登记其余三个可查询配置 Bean。
        let property = Arc::new(LiteflowProperty::default());
        let monitor_property = Arc::new(LiteflowMonitorProperty::default());
        context.register_typed_bean("liteflowProperty", Arc::clone(&property));
        context.register_typed_bean("liteflowMonitorProperty", Arc::clone(&monitor_property));
        context.register_typed_bean(
            "liteflowAutoConfiguration",
            Arc::new(LiteflowAutoConfiguration::new(
                monitor_property.is_enable_log(),
            )),
        );

        for registration in registrations {
            if let Some(node_component) = registration.managed_component() {
                // 对应 subWrapsOfType(NodeComponent)：Bean 名即默认节点 ID，同一 Arc
                // 同时进入上下文节点表和 Holder。
                context.register_node_component(registration.component_id(), node_component);
                node_id_holder.add(registration.component_id());
                continue;
            }

            if let Some(decl_warp_bean) = registration.decl_warp_bean() {
                // Rust 过程宏已经完成类级去重；此处仍经 Solon 专属解析器校验并
                // 为每个声明节点创建真实代理。
                for parsed in decl_component_parser.parse_decl_bean(decl_warp_bean)? {
                    let node_id = parsed.node_id().to_string();
                    context.register_decl_wrap_bean(&node_id, parsed.clone());
                    LiteFlowProxyUtil::register_decl_warp(flow_bus, parsed)?;
                }
                continue;
            }

            if registration.cmp_around_aspect_instance().is_some() {
                // 全局切面由 SolonCmpAroundAspect 统一接线，避免同一业务切面被
                // FlowBus 重复注册。
                continue;
            }

            // 生命周期、脚本 Bean、脚本方法及调用方自定义注册动作都保留其
            // 原始闭包逻辑；LifeCycle 注册最终进入当前 FlowBus 隔离持有器。
            registration.apply(flow_bus)?;
        }

        self.started.store(true, Ordering::Release);
        Ok(())
    }

    /// 返回默认配置是否已经加载。
    ///
    /// # 返回
    /// `start` 至少执行到默认属性注册阶段后返回 `true`。
    #[must_use]
    pub fn is_default_properties_loaded(&self) -> bool {
        self.default_properties_loaded.load(Ordering::Acquire)
    }

    /// 返回插件是否完成启用状态下的启动。
    ///
    /// # 返回
    /// 禁用时保持 `false`；全部注册成功后返回 `true`。
    #[must_use]
    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }
}
