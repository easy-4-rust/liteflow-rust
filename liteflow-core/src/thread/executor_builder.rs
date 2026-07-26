//! 并行执行器构建契约及 Tokio 运行时载体。

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::{Notify, Semaphore};

use crate::exception::{LFResult, LiteflowError};

/// Java `ExecutorService` 的 Rust 异步运行时载体。
///
/// 这不是额外迁移的 Java 业务对象，而是 `ExecutorBuilder` 返回值在 Rust 中的
/// 具体表示：worker 信号量限制并发数，admission 信号量覆盖运行中任务与有界队列；
/// 队列耗尽时调用 Future 产生背压，对应 Java `CallerRunsPolicy` 的反压效果。
pub struct ExecutorService {
    core_pool_size: usize,
    maximum_pool_size: usize,
    queue_capacity: usize,
    thread_name: String,
    workers: Arc<Semaphore>,
    admission: Arc<Semaphore>,
    closed: AtomicBool,
    active: AtomicUsize,
    idle: Notify,
}

impl ExecutorService {
    /// 创建有界异步执行器。
    ///
    /// 参数分别对应 Java `ThreadPoolExecutor` 的 corePoolSize、
    /// maximumPoolSize、queueCapacity 与线程名前缀。
    #[must_use]
    pub fn new(
        core_pool_size: usize,
        maximum_pool_size: usize,
        queue_capacity: usize,
        thread_name: impl Into<String>,
    ) -> Self {
        let maximum_pool_size = maximum_pool_size.max(1);
        let core_pool_size = core_pool_size.clamp(1, maximum_pool_size);
        Self {
            core_pool_size,
            maximum_pool_size,
            queue_capacity,
            thread_name: thread_name.into(),
            workers: Arc::new(Semaphore::new(maximum_pool_size)),
            admission: Arc::new(Semaphore::new(
                maximum_pool_size.saturating_add(queue_capacity),
            )),
            closed: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            idle: Notify::new(),
        }
    }

    /// 在执行器的并发和排队边界内运行 Future。
    ///
    /// admission permit 表示 Java 有界队列中的槽位，worker permit 表示实际工作
    /// 线程；任务完成或取消时 RAII guard 会可靠归还计数。
    pub async fn execute<F, T>(&self, future: F) -> LFResult<T>
    where
        F: Future<Output = T>,
    {
        if self.is_shutdown() {
            return Err(LiteflowError::ThreadExecutorServiceCreate(format!(
                "executor[{}] has been shut down",
                self.thread_name
            )));
        }
        let _admission = self.admission.acquire().await.map_err(|_| {
            LiteflowError::ThreadExecutorServiceCreate(format!(
                "executor[{}] admission queue is closed",
                self.thread_name
            ))
        })?;
        let _worker = self.workers.acquire().await.map_err(|_| {
            LiteflowError::ThreadExecutorServiceCreate(format!(
                "executor[{}] worker pool is closed",
                self.thread_name
            ))
        })?;
        self.active.fetch_add(1, Ordering::AcqRel);
        let _active_guard = ActiveTaskGuard { service: self };
        Ok(future.await)
    }

    /// 停止接收新任务；已获得 worker permit 的任务继续运行。
    ///
    /// 对应 Java: `ExecutorService#shutdown`。
    pub fn shutdown(&self) {
        self.closed.store(true, Ordering::Release);
        self.admission.close();
        self.workers.close();
        self.idle.notify_waiters();
    }

    /// 在指定时限内等待活动任务结束。
    ///
    /// 返回 `true` 表示全部活动任务已经完成。对应 Java:
    /// `ExecutorService#awaitTermination`。
    pub async fn await_termination(&self, timeout: Duration) -> bool {
        let wait = async {
            loop {
                let notified = self.idle.notified();
                if self.active_count() == 0 {
                    return;
                }
                notified.await;
            }
        };
        tokio::time::timeout(timeout, wait).await.is_ok()
    }

    /// 返回核心并发数。
    #[must_use]
    pub fn core_pool_size(&self) -> usize {
        self.core_pool_size
    }

    /// 返回最大并发数。
    #[must_use]
    pub fn maximum_pool_size(&self) -> usize {
        self.maximum_pool_size
    }

    /// 返回等待队列容量。
    #[must_use]
    pub fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    /// 返回 Java 兼容线程名前缀；Tokio 任务本身不绑定固定 OS 线程。
    #[must_use]
    pub fn thread_name(&self) -> &str {
        &self.thread_name
    }

    /// 返回正在运行的任务数。
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }

    /// 返回执行器是否已进入关闭状态。
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

struct ActiveTaskGuard<'a> {
    service: &'a ExecutorService,
}

impl Drop for ActiveTaskGuard<'_> {
    fn drop(&mut self) {
        self.service.active.fetch_sub(1, Ordering::AcqRel);
        self.service.idle.notify_waiters();
    }
}

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
