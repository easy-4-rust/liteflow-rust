//! LiteFlow 默认全局执行器构建器。

use std::sync::Arc;

use super::{ExecutorBuilder, ExecutorHelper, ExecutorService};

/// 为 WHEN 与异步循环构建默认全局执行器。
///
/// 对应 Java:
/// `com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder`。
pub struct LiteFlowDefaultGlobalExecutorBuilder;

impl LiteFlowDefaultGlobalExecutorBuilder {
    /// Java 配置与规则中使用的稳定构建器类名。
    pub const CLASS_NAME: &'static str =
        "com.yomahub.liteflow.thread.LiteFlowDefaultGlobalExecutorBuilder";
}

impl ExecutorBuilder for LiteFlowDefaultGlobalExecutorBuilder {
    /// 使用当前全局线程数、队列容量和 `global-thread-` 前缀构建执行器。
    ///
    /// 对应 Java: `LiteFlowDefaultGlobalExecutorBuilder#buildExecutor`。
    fn build_executor(&self) -> Arc<ExecutorService> {
        let helper = ExecutorHelper::load_instance();
        let pool_size = helper.global_thread_pool_size();
        self.build_default_executor(
            pool_size,
            pool_size,
            helper.global_thread_pool_queue_size(),
            "global-thread-",
        )
    }
}
