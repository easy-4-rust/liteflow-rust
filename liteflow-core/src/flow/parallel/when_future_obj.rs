//! 对应 Java 类：com.yomahub.liteflow.flow.parallel.WhenFutureObj
//!
//! 并行异步任务（Java 中为 CompletableFuture）里的值对象：
//! success / timeout / executorName / ex 四字段与 Java 对齐。
//! Java 的 ex 为 java.lang.Exception，Rust 端对应为 LiteflowError。

use crate::exception::LiteflowError;
use crate::exception::when_timeout_exception::WhenTimeoutException;

/// 并行任务结果载体（对应 WhenFutureObj）
#[derive(Debug, Clone)]
pub struct WhenFutureObj {
    /// 是否执行成功
    pub success: bool,
    /// 是否超时（对应 WhenCondition 的 when-max-timeout 语义）
    pub timeout: bool,
    /// 执行项标识（Java 取 executableItem.getExecuteId()）
    pub executor_name: String,
    /// 失败/超时时的异常（对应 ex 字段）
    pub ex: Option<LiteflowError>,
}

impl WhenFutureObj {
    /// 对应 success(executorName)
    pub fn success(executor_name: impl Into<String>) -> Self {
        Self {
            success: true,
            timeout: false,
            executor_name: executor_name.into(),
            ex: None,
        }
    }

    /// 对应 fail(executorName, ex)
    pub fn fail(executor_name: impl Into<String>, ex: LiteflowError) -> Self {
        Self {
            success: false,
            timeout: false,
            executor_name: executor_name.into(),
            ex: Some(ex),
        }
    }

    /// 对应 timeOut(executorName)：超时结果，ex 为 WhenTimeoutException。
    /// （Java 的 message 中会拼接 LiteflowConfigGetter 的 when-max-timeout-seconds
    /// 配置值；Rust 端超时配置由调用处显式传入，此处仅保留组件标识。）
    pub fn time_out(executor_name: impl Into<String>) -> Self {
        let name = executor_name.into();
        Self {
            success: false,
            timeout: true,
            executor_name: name.clone(),
            ex: Some(
                WhenTimeoutException::new(format!(
                    "Timed out when executing the component[{name}]"
                ))
                .into(),
            ),
        }
    }

    /// 对应 isSuccess()
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// 对应 isTimeout()
    pub fn is_timeout(&self) -> bool {
        self.timeout
    }
}
