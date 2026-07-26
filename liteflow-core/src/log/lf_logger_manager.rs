//! 对应 Java: com.yomahub.liteflow.log.LFLoggerManager

use std::cell::RefCell;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;

use super::LFLog;

thread_local! {
    static THREAD_REQUEST_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

tokio::task_local! {
    static TASK_REQUEST_ID: String;
}

fn loggers() -> &'static DashMap<String, Arc<LFLog>> {
    static LOGGERS: OnceLock<DashMap<String, Arc<LFLog>>> = OnceLock::new();
    LOGGERS.get_or_init(DashMap::new)
}

static PRINT_EXECUTION_LOG: AtomicBool = AtomicBool::new(true);

/// 日志包装器缓存与请求 id 上下文管理器。
pub struct LFLoggerManager;

impl LFLoggerManager {
    /// 获取或创建指定 target 的日志包装器。
    #[must_use]
    pub fn get_logger(target: impl Into<String>) -> Arc<LFLog> {
        let target = target.into();
        loggers()
            .entry(target.clone())
            .or_insert_with(|| Arc::new(LFLog::new(target)))
            .clone()
    }

    /// 设置当前线程的请求 id。
    pub fn set_request_id(request_id: impl Into<String>) {
        THREAD_REQUEST_ID.with(|current| *current.borrow_mut() = Some(request_id.into()));
    }

    /// 返回任务级请求 id；任务未设置时回退到线程级值。
    #[must_use]
    pub fn get_request_id() -> Option<String> {
        TASK_REQUEST_ID
            .try_with(Clone::clone)
            .ok()
            .or_else(|| THREAD_REQUEST_ID.with(|current| current.borrow().clone()))
    }

    /// 删除当前线程的请求 id。
    pub fn remove_request_id() {
        THREAD_REQUEST_ID.with(|current| *current.borrow_mut() = None);
    }

    /// 在异步任务作用域中传播请求 id。
    pub async fn scope_request_id<F, T>(request_id: impl Into<String>, future: F) -> T
    where
        F: Future<Output = T>,
    {
        TASK_REQUEST_ID.scope(request_id.into(), future).await
    }

    /// 配置 INFO/WARN/ERROR 是否输出，对接 Vernal `print_execution_log`。
    pub fn set_print_execution_log(enabled: bool) {
        PRINT_EXECUTION_LOG.store(enabled, Ordering::Release);
    }

    /// 返回执行日志开关。
    #[must_use]
    pub fn is_print_execution_log() -> bool {
        PRINT_EXECUTION_LOG.load(Ordering::Acquire)
    }
}
