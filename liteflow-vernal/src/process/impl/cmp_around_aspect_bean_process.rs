use liteflow_core::LFResult;

use crate::LiteflowComponentRegistration;
use crate::process::LiteflowScannerProcessStep;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::enums::LiteflowScannerProcessStepEnum;

/// 处理全局组件切面 Bean。
///
/// 匹配后将同一业务切面保存到上下文 Holder，供 Vernal SPI 适配器委托执行。
/// 对应 Java:
/// `com.yomahub.liteflow.spring.process.impl.CmpAroundAspectBeanProcess`。
pub struct CmpAroundAspectBeanProcess;

impl CmpAroundAspectBeanProcess {
    /// 创建组件切面处理器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CmpAroundAspectBeanProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteflowScannerProcessStep for CmpAroundAspectBeanProcess {
    fn filter(&self, context: &mut LiteflowScannerProcessStepContext<'_>) -> bool {
        context
            .registration()
            .cmp_around_aspect_instance()
            .is_some()
    }

    fn post_process_after_initialization(
        &self,
        context: &mut LiteflowScannerProcessStepContext<'_>,
    ) -> LFResult<LiteflowComponentRegistration> {
        if let Some(instance) = context.registration().cmp_around_aspect_instance() {
            context.spring_cmp_around_aspect_holder().init(instance);
        }
        Ok(context.registration().clone())
    }

    fn step_type(&self) -> LiteflowScannerProcessStepEnum {
        LiteflowScannerProcessStepEnum::CmpAroundAspectBean
    }
}
