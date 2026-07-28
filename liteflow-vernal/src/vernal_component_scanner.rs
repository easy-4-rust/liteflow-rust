//! 对应 Java: com.yomahub.liteflow.spring.ComponentScanner

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use liteflow_core::util::LOGOPrinter;
use liteflow_core::{FlowBus, LFResult};

use crate::process::LiteflowScannerProcessStepFactory;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::holder::{SpringCmpAroundAspectHolder, SpringNodeIdHolder};
use crate::{LiteflowComponentRegistration, LiteflowConfig, LiteflowVernalError};

/// Vernal 组件扫描后处理器。
///
/// Java 通过 `BeanPostProcessor` 扫描 Spring Bean；Vernal 采用显式模块注册，
/// 因此扫描对象消费 `liteflow-derive` 或应用模块提交的强类型注册定义，并保持
/// 相同的“初始化前原样返回、初始化后只执行第一个匹配处理路径”时序。托管节点
/// 延迟交给 `VernalContextCmpInit`，其他定义立即进入真实 `FlowBus`。
///
/// 对应 Java: `com.yomahub.liteflow.spring.ComponentScanner`。
pub struct VernalComponentScanner {
    registrations: Vec<LiteflowComponentRegistration>,
    scanned_component_ids: RwLock<HashSet<String>>,
    process_step_factory: Arc<LiteflowScannerProcessStepFactory>,
    spring_node_id_holder: Arc<SpringNodeIdHolder>,
    spring_cmp_around_aspect_holder: Arc<SpringCmpAroundAspectHolder>,
}

impl VernalComponentScanner {
    /// 创建扫描器并按 Java 默认构造器打印 LiteFlow 标识。
    ///
    /// # 参数
    /// - `registrations`：待扫描的显式组件定义。
    ///
    /// # 返回
    /// 拥有独立扫描缓存的组件后处理器。对应 Java:
    /// `ComponentScanner#ComponentScanner()`。
    #[must_use]
    pub fn new(registrations: Vec<LiteflowComponentRegistration>) -> Self {
        LOGOPrinter::print();
        Self::from_registrations(registrations)
    }

    /// 使用 LiteFlow 配置创建扫描器。
    ///
    /// # 参数
    /// - `liteflow_config`：提供 `printBanner` 开关的 Vernal 配置；
    /// - `registrations`：待扫描的显式组件定义。
    ///
    /// # 返回
    /// 配置禁止标识时不打印，其他行为与默认构造器一致。对应 Java:
    /// `ComponentScanner#ComponentScanner(LiteflowConfig)`。
    #[must_use]
    pub fn with_config(
        liteflow_config: &LiteflowConfig,
        registrations: Vec<LiteflowComponentRegistration>,
    ) -> Self {
        if liteflow_config.print_banner {
            LOGOPrinter::print();
        }
        Self::from_registrations(registrations)
    }

    /// 在组件初始化前保留原始定义。
    ///
    /// # 参数
    /// - `registration`：当前强类型组件定义。
    ///
    /// # 返回
    /// 与输入共享相同注册逻辑的克隆。对应 Java:
    /// `ComponentScanner#postProcessBeforeInitialization`。
    #[must_use]
    pub fn post_process_before_initialization(
        &self,
        registration: &LiteflowComponentRegistration,
    ) -> LiteflowComponentRegistration {
        registration.clone()
    }

    /// 在组件初始化后执行对应注册路径。
    ///
    /// # 参数
    /// - `flow_bus`：接收普通或声明式组件的真实流程总线；
    /// - `registration`：当前组件定义。
    ///
    /// # 返回
    /// 托管节点返回其 ID 与共享实例，供 `VernalContextCmpInit` 延迟初始化；
    /// 其他定义立即注册并返回 `None`。对应 Java:
    /// `ComponentScanner#postProcessAfterInitialization`。
    pub fn post_process_after_initialization(
        &self,
        flow_bus: &FlowBus,
        registration: &LiteflowComponentRegistration,
    ) -> LFResult<Option<LiteflowComponentRegistration>> {
        let mut context = LiteflowScannerProcessStepContext::of(
            registration.clone(),
            flow_bus,
            Arc::clone(&self.spring_node_id_holder),
            Arc::clone(&self.spring_cmp_around_aspect_holder),
        );
        let mut processed_registration = None;

        // Java 扫描链在首个 filter 命中后立即 break；这里保留完全相同的短路顺序。
        for process_step in self.process_step_factory.get_process_steps() {
            if process_step.filter(&mut context) {
                processed_registration =
                    Some(process_step.post_process_after_initialization(&mut context)?);
                break;
            }
        }

        let processed_registration = if let Some(processed_registration) = processed_registration {
            processed_registration
        } else {
            // 显式自定义注册没有对应 Java 类型时仍执行原有注册闭包。
            registration.apply(flow_bus)?;
            registration.clone()
        };
        let managed_component = processed_registration.managed_component();
        let component_id = processed_registration.component_id().to_string();
        self.scanned_component_ids
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(component_id.clone());
        Ok(managed_component.map(|_| processed_registration))
    }

    /// 扫描当前模块的全部组件定义。
    ///
    /// # 参数
    /// - `flow_bus`：接收非托管组件的真实流程总线。
    ///
    /// # 返回
    /// 需要由容器初始化 SPI 统一处理的托管注册定义；注册失败时携带组件 ID。
    /// 对应 Java: `ComponentScanner#postProcessAfterInitialization` 的逐 Bean 调用。
    pub fn scan(
        &self,
        flow_bus: &FlowBus,
    ) -> Result<Vec<LiteflowComponentRegistration>, LiteflowVernalError> {
        let mut managed_registrations = Vec::new();
        for registration in &self.registrations {
            let registration = self.post_process_before_initialization(registration);
            if let Some(managed_registration) = self
                .post_process_after_initialization(flow_bus, &registration)
                .map_err(|error| LiteflowVernalError::ComponentRegistration {
                    component_id: registration.component_id().to_string(),
                    message: error.to_string(),
                })?
            {
                managed_registrations.push(managed_registration);
            }
        }
        Ok(managed_registrations)
    }

    /// 返回已经成功扫描的组件 ID 快照。
    ///
    /// # 返回
    /// 按字典序排序的组件 ID，便于启动诊断和确定性测试。
    #[must_use]
    pub fn scanned_component_ids(&self) -> Vec<String> {
        let mut component_ids: Vec<_> = self
            .scanned_component_ids
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .cloned()
            .collect();
        component_ids.sort();
        component_ids
    }

    /// 返回扫描步骤工厂。
    ///
    /// # 返回
    /// 当前应用上下文独享且按 Java 优先级排序的工厂实例。
    #[must_use]
    pub fn process_step_factory(&self) -> Arc<LiteflowScannerProcessStepFactory> {
        Arc::clone(&self.process_step_factory)
    }

    /// 返回节点 ID 持有器。
    ///
    /// # 返回
    /// 包含本轮成功扫描节点的上下文级 Holder。
    #[must_use]
    pub fn spring_node_id_holder(&self) -> Arc<SpringNodeIdHolder> {
        Arc::clone(&self.spring_node_id_holder)
    }

    /// 返回组件切面持有器。
    ///
    /// # 返回
    /// 包含扫描到业务切面的上下文级 Holder。
    #[must_use]
    pub fn spring_cmp_around_aspect_holder(&self) -> Arc<SpringCmpAroundAspectHolder> {
        Arc::clone(&self.spring_cmp_around_aspect_holder)
    }

    /// 清理当前应用上下文的扫描缓存。
    ///
    /// Java 使用进程级静态集合；Vernal 把缓存绑定到真实扫描器单例，避免多个
    /// `ApplicationContext` 相互清理。对应 Java: `ComponentScanner#cleanCache`。
    pub fn clean_cache(&self) {
        self.scanned_component_ids
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        self.spring_node_id_holder.clean();
        self.spring_cmp_around_aspect_holder.clean();
    }

    fn from_registrations(registrations: Vec<LiteflowComponentRegistration>) -> Self {
        Self {
            registrations,
            scanned_component_ids: RwLock::new(HashSet::new()),
            process_step_factory: Arc::new(LiteflowScannerProcessStepFactory::new()),
            spring_node_id_holder: Arc::new(SpringNodeIdHolder::new()),
            spring_cmp_around_aspect_holder: Arc::new(SpringCmpAroundAspectHolder::new()),
        }
    }
}
