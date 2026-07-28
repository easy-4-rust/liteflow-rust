use liteflow_core::LFResult;

use crate::LiteflowComponentRegistration;
use crate::process::LiteflowScannerProcessStep;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::enums::LiteflowScannerProcessStepEnum;

/// 处理 LiteFlow 生命周期扩展 Bean。
///
/// 匹配后把真实生命周期对象加入当前 FlowBus 的隔离 `LifeCycleHolder`，后续
/// 规则加载和执行会调用相应阶段。对应 Java:
/// `com.yomahub.liteflow.spring.process.impl.LifeCycleBeanProcess`。
pub struct LifeCycleBeanProcess;

impl LifeCycleBeanProcess {
    /// 创建生命周期处理器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LifeCycleBeanProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteflowScannerProcessStep for LifeCycleBeanProcess {
    fn filter(&self, context: &mut LiteflowScannerProcessStepContext<'_>) -> bool {
        context.registration().life_cycle_instance().is_some()
    }

    fn post_process_after_initialization(
        &self,
        context: &mut LiteflowScannerProcessStepContext<'_>,
    ) -> LFResult<LiteflowComponentRegistration> {
        if let Some(life_cycle) = context.registration().life_cycle_instance() {
            context.flow_bus().register_life_cycle(life_cycle);
        }
        Ok(context.registration().clone())
    }

    fn step_type(&self) -> LiteflowScannerProcessStepEnum {
        LiteflowScannerProcessStepEnum::LifeCycleBean
    }
}
