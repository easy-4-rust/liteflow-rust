use liteflow_core::LFResult;

use crate::LiteflowComponentRegistration;
use crate::process::LiteflowScannerProcessStep;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::enums::LiteflowScannerProcessStepEnum;

/// 处理声明式组件包装对象。
///
/// 声明定义已由 `VernalDeclBeanDefinition` 拆分校验；本步骤生成真实代理并写入
/// FlowBus，同时记录最终节点 ID。对应 Java:
/// `com.yomahub.liteflow.spring.process.impl.DeclWarpBeanProcess`。
pub struct DeclWarpBeanProcess;

impl DeclWarpBeanProcess {
    /// 创建声明式组件处理器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for DeclWarpBeanProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteflowScannerProcessStep for DeclWarpBeanProcess {
    fn filter(&self, context: &mut LiteflowScannerProcessStepContext<'_>) -> bool {
        context.registration().decl_warp_bean().is_some()
    }

    fn post_process_after_initialization(
        &self,
        context: &mut LiteflowScannerProcessStepContext<'_>,
    ) -> LFResult<LiteflowComponentRegistration> {
        let registration = context.registration().clone();
        registration.apply(context.flow_bus())?;
        let node_id = registration.decl_warp_bean().map_or_else(
            || context.bean_name().to_string(),
            |bean| bean.node_id().to_string(),
        );
        context.spring_node_id_holder().add_node_id(node_id);
        Ok(registration)
    }

    fn step_type(&self) -> LiteflowScannerProcessStepEnum {
        LiteflowScannerProcessStepEnum::DeclWarpBean
    }
}
