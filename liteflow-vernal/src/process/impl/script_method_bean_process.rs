use std::sync::Arc;

use liteflow_core::LFResult;
use liteflow_core::script::ScriptBeanManager;
use liteflow_core::script::proxy::ScriptBeanProxy;

use crate::LiteflowComponentRegistration;
use crate::process::LiteflowScannerProcessStep;
use crate::process::context::LiteflowScannerProcessStepContext;
use crate::process::enums::LiteflowScannerProcessStepEnum;

/// 处理 `@ScriptMethod` 等价方法分组。
///
/// Java 在过滤阶段反射收集非空 value 的方法；Rust 宏/应用模块在注册定义构建期
/// 完成分组，本步骤保留同一 filter/outPut/postProcess 时序并注册每组真实代理。
/// 对应 Java:
/// `com.yomahub.liteflow.spring.process.impl.ScriptMethodBeanProcess`。
pub struct ScriptMethodBeanProcess;

impl ScriptMethodBeanProcess {
    /// 创建脚本方法处理器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ScriptMethodBeanProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteflowScannerProcessStep for ScriptMethodBeanProcess {
    fn filter(&self, context: &mut LiteflowScannerProcessStepContext<'_>) -> bool {
        if let Some(proxies) = context.registration().script_method_proxies() {
            if proxies.is_empty() {
                return false;
            }
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
        LiteflowScannerProcessStepEnum::ScriptMethodBean
    }
}
