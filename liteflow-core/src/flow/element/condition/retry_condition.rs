//! RETRY 条件：首次执行失败后最多重试 `retryTimes` 次。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 对单个可执行对象应用异常过滤和有限次数重试的条件。
///
/// Java 继承 `ThenCondition` 并约束默认分组只有一个可执行对象；Rust 直接持有该
/// 对象，保持相同执行约束。对应 Java:
/// `com.yomahub.liteflow.flow.element.condition.RetryCondition`。
pub struct RetryCondition {
    inner: Arc<dyn Executable>,
    retry_times: u32,
    retry_for_exceptions: Vec<String>,
}

impl RetryCondition {
    /// 创建使用 Java 默认异常范围的重试条件。
    ///
    /// 参数 `inner` 是唯一可执行对象，`retry_times` 是失败后的最大重试次数。
    /// 对应 Java: `RetryCondition` 默认 `Exception.class` 范围。
    pub fn new(inner: Arc<dyn Executable>, retry_times: u32) -> Self {
        Self {
            inner,
            retry_times,
            retry_for_exceptions: vec!["Exception".to_string()],
        }
    }

    /// 创建带错误类型过滤的重试条件。
    /// 对应 Java: `RetryCondition#setRetryForExceptions`。
    pub fn with_exceptions(
        inner: Arc<dyn Executable>,
        retry_times: u32,
        retry_for_exceptions: Vec<String>,
    ) -> Self {
        Self {
            inner,
            retry_times,
            retry_for_exceptions,
        }
    }

    /// 返回允许触发重试的异常类型名称。
    ///
    /// Rust 使用稳定异常名称代替 Java `Class<? extends Exception>[]`。对应 Java:
    /// `RetryCondition#getRetryForExceptions`。
    #[must_use]
    pub fn get_retry_for_exceptions(&self) -> &[String] {
        &self.retry_for_exceptions
    }

    /// 设置允许触发重试的异常类型名称。
    ///
    /// 参数 `retry_for_exceptions` 对应 Java 同名参数；空列表表示任何异常都不
    /// 重试。对应 Java: `RetryCondition#setRetryForExceptions`。
    pub fn set_retry_for_exceptions(&mut self, retry_for_exceptions: Vec<String>) {
        self.retry_for_exceptions = retry_for_exceptions;
    }

    /// 返回失败后的最大重试次数。对应 Java: `RetryCondition#getRetryTimes`。
    #[must_use]
    pub fn get_retry_times(&self) -> u32 {
        self.retry_times
    }

    /// 设置失败后的最大重试次数。
    ///
    /// Java 在执行时把负数视为 0；Rust 在写入时完成同样归一化。参数
    /// `retry_times` 对应 Java 同名参数。对应 Java:
    /// `RetryCondition#setRetryTimes`。
    pub fn set_retry_times(&mut self, retry_times: i32) {
        self.retry_times = retry_times.max(0) as u32;
    }

    fn matches_retry_filter(&self, error: &LiteflowError) -> bool {
        if self.retry_for_exceptions.is_empty() {
            return false;
        }
        let debug;
        let variant = match error {
            LiteflowError::NodeExec { kind, .. } => kind.as_str(),
            other => {
                debug = format!("{other:?}");
                debug.split([' ', '(', '{']).next().unwrap_or_default()
            }
        }
        .trim_end_matches("Exception");
        self.retry_for_exceptions.iter().any(|name| {
            let simple = name.rsplit('.').next().unwrap_or(name);
            simple == "Exception"
                || simple.trim_end_matches("Exception") == variant
                || simple == "LiteflowError"
                || simple == "Error"
        })
    }

    /// 执行唯一子对象，并按异常类型最多重试指定次数。
    ///
    /// 参数 `ctx`、`frame` 对应 Java `slotIndex` 定位的 Slot 和线程状态。重试前
    /// 清除上一次写入 Slot 的异常；`ChainEnd` 始终直接透传。返回最后一次成功值
    /// 或最终异常。对应 Java: `RetryCondition#executeCondition`。
    pub async fn execute_condition(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        for retry_time in 0..=self.retry_times {
            match self.inner.execute(ctx, frame).await {
                Ok(value) => return Ok(value),
                Err(LiteflowError::ChainEnd(message)) => {
                    return Err(LiteflowError::ChainEnd(message));
                }
                Err(error)
                    if self.matches_retry_filter(&error) && retry_time < self.retry_times =>
                {
                    // Java 在下一次重试前删除 Slot 异常，避免成功后仍残留旧错误。
                    ctx.inner.remove_exception();
                }
                Err(error) => return Err(error),
            }
        }
        Ok(Value::Null)
    }
}

#[async_trait]
impl Executable for RetryCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        self.execute_condition(ctx, frame).await
    }

    fn id(&self) -> &str {
        "RETRY"
    }
}
