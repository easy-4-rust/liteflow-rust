use liteflow_core::LFResult;

use crate::LiteflowComponentRegistration;
use crate::process::LiteflowScannerProcessStep;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::enums::LiteflowScannerProcessStepEnum;

/// 处理由 Vernal 容器托管的普通节点组件。
///
/// 本步骤只记录真实 Bean 名并保留注册定义，节点实例随后由
/// `VernalContextCmpInit` 在规则解析前统一注册。对应 Java:
/// `com.yomahub.liteflow.spring.process.impl.NodeCmpBeanProcess`。
pub struct NodeCmpBeanProcess;

impl NodeCmpBeanProcess {
    /// 创建普通节点处理器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for NodeCmpBeanProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteflowScannerProcessStep for NodeCmpBeanProcess {
    fn filter(&self, context: &mut LiteflowScannerProcessStepContext<'_>) -> bool {
        context.registration().managed_component().is_some()
    }

    fn post_process_after_initialization(
        &self,
        context: &mut LiteflowScannerProcessStepContext<'_>,
    ) -> LFResult<LiteflowComponentRegistration> {
        let node_id = context
            .spring_node_id_holder()
            .get_real_bean_name_with_scope(
                context.clazz(),
                context.bean_name(),
                context.registration().is_refresh_scoped(),
            );
        context.spring_node_id_holder().add_node_id(node_id.clone());
        let node_component = context
            .registration()
            .managed_component()
            .expect("filter 已保证托管节点实例存在");
        // Java 对 RefreshScope 代理使用去除 scopedTarget. 后的真实 Bean 名；
        // 重建注册动作可以让后续 ContextCmpInit 使用完全相同的节点 ID。
        Ok(LiteflowComponentRegistration::managed(
            node_id,
            node_component,
        ))
    }

    fn step_type(&self) -> LiteflowScannerProcessStepEnum {
        LiteflowScannerProcessStepEnum::NodeCmpBean
    }
}
