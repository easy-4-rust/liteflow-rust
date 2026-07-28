//! 对应 TimeoutCondition：MAX_WAIT_SECONDS / MAX_WAIT_MILLISECONDS。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// 为单个可执行对象施加最大等待时间的条件。
///
/// 对应 Java: `com.yomahub.liteflow.flow.element.condition.TimeoutCondition`。
pub struct TimeoutCondition {
    inner: Arc<dyn Executable>,
    max_wait_ms: u64,
}

impl TimeoutCondition {
    /// 使用内部可执行对象和毫秒级最大等待时间创建条件。
    ///
    /// 参数 `inner` 为被限制的执行对象，`max_wait_ms` 为最大等待毫秒数。
    /// 对应 Java: `TimeoutCondition#TimeoutCondition`。
    #[must_use]
    pub fn new(inner: Arc<dyn Executable>, max_wait_ms: u64) -> Self {
        Self { inner, max_wait_ms }
    }

    /// 在最大等待时间内执行内部对象。
    ///
    /// 参数 `ctx` 与 `frame` 对应 Java `slotIndex` 定位的执行状态；按时完成时返回
    /// 原始结果或错误，超时返回 `WhenTimeout`。
    /// 对应 Java: `TimeoutCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        match tokio::time::timeout(
            Duration::from_millis(self.max_wait_ms),
            self.inner.execute(ctx, frame),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(LiteflowError::WhenTimeout("when timeout".to_string())),
        }
    }
}

#[async_trait]
impl Executable for TimeoutCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        self.execute_condition(ctx, frame).await
    }

    fn id(&self) -> &str {
        "TIMEOUT"
    }
}
