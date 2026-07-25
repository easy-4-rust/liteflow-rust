//! 对应 Java 类：com.yomahub.liteflow.flow.parallel.CompletableFutureTimeout
//!
//! 带超时的 future 包装。
//!
//! 与 Java 实现的差异说明：
//! - Java（timeoutAfter / completeOnTimeout）基于 ScheduledThreadPoolExecutor 守护线程，
//!   超时后让 CompletableFuture 以 TimeoutException 异步完成，**原任务仍在后台线程
//!   继续执行**，只是其结果无人接收；
//! - Rust 端基于 tokio::time::timeout：超时即 drop 原 future（协作式取消，
//!   原任务在下一个 await 点被终止），超时分支直接返回默认值。
//!   语义对齐 Java completeOnTimeout（超时兜底为默认值），
//!   而 orTimeout 的「异常完成但任务继续」语义在 tokio 中应直接用
//!   tokio::spawn + tokio::time::timeout 表达，本模块不再单独提供。

use std::future::Future;
use std::time::Duration;

/// 对应 completeOnTimeout(T t, CompletableFuture<T> future, long timeout, TimeUnit unit)：
/// future 在 timeout 内完成则返回其结果；超时则返回默认值 `default`
/// （Java 中是 applyToEither 先拿到 timeoutFuture 的 TimeoutException，
/// 再 exceptionally 兜底为 t；Rust 端由 tokio::time::timeout 的 Err 分支直接给出默认值）。
pub async fn complete_on_timeout<T, F>(default: T, future: F, timeout: Duration) -> T
where
    F: Future<Output = T>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(v) => v,
        Err(_) => default,
    }
}
