use std::any::Any;
use std::sync::Arc;

use liteflow_core::FlowBus;

use crate::LiteflowComponentRegistration;
use crate::process::holder::{SpringCmpAroundAspectHolder, SpringNodeIdHolder};

/// 单个容器 Bean 在扫描步骤间传递的上下文。
///
/// Java 保存 bean、beanName、clazz 与可变 outPut；Rust 以强类型注册定义承载
/// Bean，并额外携带当前 FlowBus 和上下文级 Holder，使处理步骤执行真实副作用。
/// 对应 Java:
/// `com.yomahub.liteflow.spring.process.context.LiteflowScannerProcessStepContext`。
pub struct LiteflowScannerProcessStepContext<'a> {
    registration: LiteflowComponentRegistration,
    bean_name: String,
    clazz: String,
    out_put: Option<Arc<dyn Any + Send + Sync>>,
    flow_bus: &'a FlowBus,
    spring_node_id_holder: Arc<SpringNodeIdHolder>,
    spring_cmp_around_aspect_holder: Arc<SpringCmpAroundAspectHolder>,
}

impl<'a> LiteflowScannerProcessStepContext<'a> {
    /// 创建扫描步骤上下文。
    ///
    /// # 参数
    /// - `registration`：当前 Bean 的强类型注册定义；
    /// - `flow_bus`：当前应用上下文的流程总线；
    /// - `spring_node_id_holder`：节点 ID 持有器；
    /// - `spring_cmp_around_aspect_holder`：组件切面持有器。
    ///
    /// # 返回
    /// 初始化了 bean、beanName 与 clazz 的上下文。对应 Java:
    /// `LiteflowScannerProcessStepContext#of`。
    #[must_use]
    pub fn of(
        registration: LiteflowComponentRegistration,
        flow_bus: &'a FlowBus,
        spring_node_id_holder: Arc<SpringNodeIdHolder>,
        spring_cmp_around_aspect_holder: Arc<SpringCmpAroundAspectHolder>,
    ) -> Self {
        Self {
            bean_name: registration.component_id().to_string(),
            clazz: registration.bean_type_name().to_string(),
            registration,
            out_put: None,
            flow_bus,
            spring_node_id_holder,
            spring_cmp_around_aspect_holder,
        }
    }

    /// 返回当前注册定义。
    #[must_use]
    pub fn registration(&self) -> &LiteflowComponentRegistration {
        &self.registration
    }

    /// 返回容器 Bean 名称。
    #[must_use]
    pub fn bean_name(&self) -> &str {
        &self.bean_name
    }

    /// 返回 Bean 类型诊断名。
    #[must_use]
    pub fn clazz(&self) -> &str {
        &self.clazz
    }

    /// 保存过滤阶段产生的中间结果。
    ///
    /// # 参数
    /// - `out_put`：后处理阶段需要复用的线程安全类型擦除对象。
    ///
    /// 对应 Java: `LiteflowScannerProcessStepContext#setOutPut`。
    pub fn set_out_put(&mut self, out_put: Arc<dyn Any + Send + Sync>) {
        self.out_put = Some(out_put);
    }

    /// 按具体类型读取过滤阶段中间结果。
    ///
    /// # 返回
    /// 类型匹配时返回共享对象，否则返回 `None`。对应 Java:
    /// `LiteflowScannerProcessStepContext#getOutPut`。
    #[must_use]
    pub fn out_put<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync,
    {
        self.out_put
            .as_ref()
            .and_then(|value| Arc::clone(value).downcast::<T>().ok())
    }

    /// 返回当前流程总线。
    #[must_use]
    pub fn flow_bus(&self) -> &FlowBus {
        self.flow_bus
    }

    /// 返回节点 ID 持有器。
    #[must_use]
    pub fn spring_node_id_holder(&self) -> &SpringNodeIdHolder {
        &self.spring_node_id_holder
    }

    /// 返回组件切面持有器。
    #[must_use]
    pub fn spring_cmp_around_aspect_holder(&self) -> &SpringCmpAroundAspectHolder {
        &self.spring_cmp_around_aspect_holder
    }
}
