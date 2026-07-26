//! FlowExecutor 全局持有器。
//!
//! 对应 Java: `com.yomahub.liteflow.core.FlowExecutorHolder`。

use std::sync::{OnceLock, RwLock};

use crate::exception::{LFResult, LiteflowError};
use crate::flow::FlowBus;

use super::FlowExecutor;

fn holder() -> &'static RwLock<Option<FlowExecutor>> {
    static FLOW_EXECUTOR: OnceLock<RwLock<Option<FlowExecutor>>> = OnceLock::new();
    FLOW_EXECUTOR.get_or_init(|| RwLock::new(None))
}

/// 为无法直接持有 FlowBus 的扩展点提供已初始化 FlowExecutor。
pub struct FlowExecutorHolder;

impl FlowExecutorHolder {
    /// 获取已初始化的执行器。
    ///
    /// 对应 Java 无参 `loadInstance()`；未初始化时返回
    /// `FlowExecutorNotInitException` 对应的错误变体。
    pub fn load_instance() -> LFResult<FlowExecutor> {
        holder().read().unwrap().clone().ok_or_else(|| {
            LiteflowError::FlowExecutorNotInit("flow executor is not initialized yet".to_string())
        })
    }

    /// 获取执行器，尚未初始化时使用给定 FlowBus 创建。
    ///
    /// 对应 Java `loadInstance(LiteflowConfig)`；Rust 的运行期配置归属于显式
    /// FlowBus，因此使用 FlowBus 作为初始化参数。
    pub fn load_instance_with_bus(bus: FlowBus) -> FlowExecutor {
        if let Some(executor) = holder().read().unwrap().clone() {
            return executor;
        }
        let executor = FlowExecutor::new(bus);
        Self::set_holder(executor.clone());
        executor
    }

    /// 替换持有的执行器。对应 Java `setHolder(FlowExecutor)`。
    pub fn set_holder(flow_executor: FlowExecutor) {
        *holder().write().unwrap() = Some(flow_executor);
    }

    /// 清空执行器。对应 Java `clean()`。
    pub fn clean() {
        *holder().write().unwrap() = None;
    }
}
