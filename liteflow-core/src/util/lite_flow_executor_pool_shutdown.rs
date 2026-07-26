//! 对应 Java: com.yomahub.liteflow.util.LiteFlowExecutorPoolShutdown

use std::sync::Arc;
use std::time::Duration;

use crate::thread::{ExecutorHelper, ExecutorService};

/// 关闭 LiteFlow WHEN 执行器并等待在途任务结束。
pub struct LiteFlowExecutorPoolShutdown;

impl LiteFlowExecutorPoolShutdown {
    /// 执行关闭清理；返回 `true` 表示在时限内全部终止。
    pub async fn destroy(executor_service: Arc<ExecutorService>, timeout: Duration) -> bool {
        ExecutorHelper::load_instance()
            .shutdown_await_termination(&executor_service, timeout)
            .await
    }
}
