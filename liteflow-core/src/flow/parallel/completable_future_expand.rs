//! 对应 Java: com.yomahub.liteflow.flow.parallel.CompletableFutureExpand

use std::future::Future;
use std::time::Duration;

/// 为 Future 提供超时默认值扩展。
pub struct CompletableFutureExpand;

impl CompletableFutureExpand {
    /// Future 在给定时限内完成则返回真实结果，超时返回默认对象。
    ///
    /// Tokio 的超时 Future 被 drop，从而协作式取消；这比 Java 守护调度线程
    /// 更符合 Rust 资源所有权模型。
    pub async fn complete_on_timeout<T, F>(
        future: F,
        timeout: Duration,
        timeout_default_obj: T,
    ) -> T
    where
        F: Future<Output = T>,
    {
        super::completable_future_timeout::complete_on_timeout(timeout_default_obj, future, timeout)
            .await
    }
}
