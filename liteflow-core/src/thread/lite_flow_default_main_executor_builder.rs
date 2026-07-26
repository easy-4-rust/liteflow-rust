//! LiteFlow 默认主执行器构建器。

use std::sync::Arc;

use super::{ExecutorBuilder, ExecutorHelper, ExecutorService};

/// 为 `FlowExecutor` 异步入口构建默认主执行器。
///
/// 对应 Java:
/// `com.yomahub.liteflow.thread.LiteFlowDefaultMainExecutorBuilder`。
pub struct LiteFlowDefaultMainExecutorBuilder;

impl LiteFlowDefaultMainExecutorBuilder {
    /// Java 配置中使用的稳定构建器类名。
    pub const CLASS_NAME: &'static str =
        "com.yomahub.liteflow.thread.LiteFlowDefaultMainExecutorBuilder";
}

impl ExecutorBuilder for LiteFlowDefaultMainExecutorBuilder {
    /// 按 Java 默认值构建 `workers / workers*2 / queue=200` 的主执行器。
    ///
    /// 对应 Java: `LiteFlowDefaultMainExecutorBuilder#buildExecutor`。
    fn build_executor(&self) -> Arc<ExecutorService> {
        let workers = ExecutorHelper::load_instance().main_executor_works();
        self.build_default_executor(workers, workers.saturating_mul(2), 200, "main-thread-")
    }
}
