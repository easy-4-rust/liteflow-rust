use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::script::ScriptBeanManager;
use liteflow_core::script::proxy::ScriptBeanProxy;

use crate::LiteflowComponentRegistration;
use crate::process::LiteflowScannerProcessStep;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::enums::LiteflowScannerProcessStepEnum;

/// 处理 `@ScriptBean` 等价注册定义。
///
/// 过滤阶段保存代理元数据，后处理阶段写入 Core 的真实 `ScriptBeanManager`。
/// 对应 Java: `com.yomahub.liteflow.spring.process.impl.ScriptBeanProcess`。
pub struct ScriptBeanProcess;

impl ScriptBeanProcess {
    /// 创建脚本 Bean 处理器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ScriptBeanProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteflowScannerProcessStep for ScriptBeanProcess {
    fn filter(&self, context: &mut LiteflowScannerProcessStepContext<'_>) -> bool {
        if let Some(proxies) = context.registration().script_bean_proxies() {
            context.set_out_put(Arc::new(proxies));
            true
        } else {
            false
        }
    }

    fn post_process_after_initialization(
        &self,
        context: &mut LiteflowScannerProcessStepContext<'_>,
    ) -> LFResult<LiteflowComponentRegistration> {
        if let Some(proxies) = context.out_put::<Vec<ScriptBeanProxy>>() {
            for proxy in proxies.iter() {
                ScriptBeanManager::add_script_bean(proxy.clone());
            }
        }
        Ok(context.registration().clone())
    }

    fn step_type(&self) -> LiteflowScannerProcessStepEnum {
        LiteflowScannerProcessStepEnum::ScriptBean
    }
}
