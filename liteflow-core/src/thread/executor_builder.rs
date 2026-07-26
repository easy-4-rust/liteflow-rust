//! 并行执行器构建契约。

use std::sync::Arc;

use super::ExecutorService;

/// 并行多任务执行器构造器接口。
///
/// Java 的反射构造由 Rust 显式注册表替代，返回的 `ExecutorService` 使用 Tokio
/// task + 信号量实现轻量任务、有界并发和背压。
///
/// 对应 Java: `com.yomahub.liteflow.thread.ExecutorBuilder`。
pub trait ExecutorBuilder: Send + Sync + 'static {
    /// 构建执行器。
    ///
    /// 对应 Java: `ExecutorBuilder#buildExecutor`。
    fn build_executor(&self) -> Arc<ExecutorService>;

    /// 构建允许 Tokio 轻量任务调度的默认执行器。
    ///
    /// 对应 Java: `ExecutorBuilder#buildDefaultExecutor`。Java 21 virtual thread
    /// 在 Rust 中由 Tokio task 承担，不创建独立 OS 线程池。
    fn build_default_executor(
        &self,
        core_pool_size: usize,
        maximum_pool_size: usize,
        queue_capacity: usize,
        thread_name: &str,
    ) -> Arc<ExecutorService> {
        Arc::new(ExecutorService::new(
            core_pool_size,
            maximum_pool_size,
            queue_capacity,
            thread_name,
        ))
    }

    /// 构建固定采用有界并发的公共执行器。
    ///
    /// 对应 Java: `ExecutorBuilder#buildCommonExecutor`。
    fn build_common_executor(
        &self,
        core_pool_size: usize,
        maximum_pool_size: usize,
        queue_capacity: usize,
        thread_name: &str,
    ) -> Arc<ExecutorService> {
        Arc::new(ExecutorService::new(
            core_pool_size,
            maximum_pool_size,
            queue_capacity,
            thread_name,
        ))
    }
}
