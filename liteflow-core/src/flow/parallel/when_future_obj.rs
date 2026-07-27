//! 对应 Java 类：com.yomahub.liteflow.flow.parallel.WhenFutureObj
//!
//! 并行异步任务（Java 中为 CompletableFuture）里的值对象：
//! success / timeout / executorId / ex 四字段与 Java 对齐。
//! Java 的 ex 为 java.lang.Exception，Rust 端对应为 LiteflowError。

use crate::exception::LiteflowError;
use crate::exception::when_timeout_exception::WhenTimeoutException;

/// 并行异步 `CompletableFuture` 的结果值对象。
///
/// 成功、超时、执行项 ID 与异常四个字段保持 Java 对象语义；字段通过 getter 和
/// setter 访问，避免调用方绕过状态封装。
///
/// 对应 Java: `com.yomahub.liteflow.flow.parallel.WhenFutureObj`。
#[derive(Debug, Clone)]
pub struct WhenFutureObj {
    success: bool,
    timeout: bool,
    executor_id: String,
    ex: Option<LiteflowError>,
}

impl WhenFutureObj {
    /// 构造执行成功结果。
    ///
    /// 参数 `executor_id` 对应 Java 同名参数；返回对象成功且未超时。
    /// 对应 Java: `WhenFutureObj#success(String)`。
    pub fn success(executor_id: impl Into<String>) -> Self {
        Self {
            success: true,
            timeout: false,
            executor_id: executor_id.into(),
            ex: None,
        }
    }

    /// 构造执行失败结果并保存原始异常。
    ///
    /// 参数 `executor_id`、`ex` 对应 Java 同名参数；返回对象失败且未超时。
    /// 对应 Java: `WhenFutureObj#fail(String, Exception)`。
    pub fn fail(executor_id: impl Into<String>, ex: LiteflowError) -> Self {
        Self {
            success: false,
            timeout: false,
            executor_id: executor_id.into(),
            ex: Some(ex),
        }
    }

    /// 构造执行超时结果，异常类型为 `WhenTimeoutException`。
    ///
    /// 参数 `executor_id` 会写入 Java 对等错误消息
    /// `Timed out when executing the component[id]`。对应 Java:
    /// `WhenFutureObj#timeOut(String)`。
    pub fn time_out(executor_id: impl Into<String>) -> Self {
        let executor_id = executor_id.into();
        Self {
            success: false,
            timeout: true,
            executor_id: executor_id.clone(),
            ex: Some(
                WhenTimeoutException::new(format!(
                    "Timed out when executing the component[{executor_id}]"
                ))
                .into(),
            ),
        }
    }

    /// 返回任务是否执行成功。对应 Java: `WhenFutureObj#isSuccess`。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// 设置任务成功状态。
    ///
    /// 参数 `success` 对应 Java 同名参数。对应 Java:
    /// `WhenFutureObj#setSuccess`。
    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }

    /// 返回执行项 ID。对应 Java: `WhenFutureObj#getExecutorId`。
    #[must_use]
    pub fn get_executor_id(&self) -> &str {
        &self.executor_id
    }

    /// 设置执行项 ID。
    ///
    /// 参数 `executor_id` 对应 Java 同名参数。对应 Java:
    /// `WhenFutureObj#setExecutorId`。
    pub fn set_executor_id(&mut self, executor_id: impl Into<String>) {
        self.executor_id = executor_id.into();
    }

    /// 返回失败或超时异常；成功结果返回 `None`。
    ///
    /// 对应 Java: `WhenFutureObj#getEx`。
    #[must_use]
    pub fn get_ex(&self) -> Option<&LiteflowError> {
        self.ex.as_ref()
    }

    /// 设置失败或超时异常。
    ///
    /// 参数 `ex` 使用 `None` 表达 Java `null`。对应 Java:
    /// `WhenFutureObj#setEx`。
    pub fn set_ex(&mut self, ex: Option<LiteflowError>) {
        self.ex = ex;
    }

    /// 返回任务是否超时。对应 Java: `WhenFutureObj#isTimeout`。
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        self.timeout
    }

    /// 设置任务超时状态。
    ///
    /// 参数 `timeout` 对应 Java 同名参数。对应 Java:
    /// `WhenFutureObj#setTimeout`。
    pub fn set_timeout(&mut self, timeout: bool) {
        self.timeout = timeout;
    }

    /// 返回执行项名称。
    ///
    /// 这是旧 Rust API 的兼容入口，委托 Java 对等 getter。
    #[must_use]
    pub fn executor_name(&self) -> &str {
        self.get_executor_id()
    }

    /// 设置执行项名称。
    ///
    /// 这是旧 Rust API 的兼容入口，委托 Java 对等 setter。
    pub fn set_executor_name(&mut self, executor_name: impl Into<String>) {
        self.set_executor_id(executor_name);
    }

    /// 返回失败异常。
    ///
    /// 这是旧 Rust API 的兼容入口，委托 Java 对等 getter。
    #[must_use]
    pub fn ex(&self) -> Option<&LiteflowError> {
        self.get_ex()
    }
}
