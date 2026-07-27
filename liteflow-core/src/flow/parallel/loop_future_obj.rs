//! 并行循环单个子项的执行结果。
//!
//! Java 的异常字段为 `Exception`；Rust 使用可克隆的 `LiteflowError`，使异步任务
//! 可以保留原始 LiteFlow 错误语义并由循环结算阶段统一处理。
//!
//! 对应 Java: `com.yomahub.liteflow.flow.parallel.LoopFutureObj`。

use crate::exception::LiteflowError;

/// 并行循环子项结果对象。
///
/// 成功时 `success` 为 true 且 `ex` 为空；失败时保留执行项名称和异常。
/// 对应 Java: `com.yomahub.liteflow.flow.parallel.LoopFutureObj`。
#[derive(Debug, Clone)]
pub struct LoopFutureObj {
    executor_name: String,
    success: bool,
    ex: Option<LiteflowError>,
}

impl LoopFutureObj {
    /// 构造成功结果。
    ///
    /// 参数 `executor_name` 是循环体可执行对象标识。对应 Java:
    /// `LoopFutureObj#success(String)`。
    pub fn success(executor_name: impl Into<String>) -> Self {
        Self {
            executor_name: executor_name.into(),
            success: true,
            ex: None,
        }
    }

    /// 构造失败结果并保留原始错误。
    ///
    /// 对应 Java: `LoopFutureObj#fail(String, Exception)`。
    pub fn fail(executor_name: impl Into<String>, ex: LiteflowError) -> Self {
        Self {
            executor_name: executor_name.into(),
            success: false,
            ex: Some(ex),
        }
    }

    /// 返回失败异常；成功结果返回 None。
    ///
    /// 对应 Java: `LoopFutureObj#getEx`。
    #[must_use]
    pub fn get_ex(&self) -> Option<&LiteflowError> {
        self.ex.as_ref()
    }

    /// 返回失败异常；成功结果返回 None。
    ///
    /// 这是既有 Rust API，委托 Java 命名入口读取同一字段。
    #[must_use]
    pub fn ex(&self) -> Option<&LiteflowError> {
        self.get_ex()
    }

    /// 返回循环体执行项名称。
    ///
    /// 对应 Java: `LoopFutureObj#getExecutorName`。
    #[must_use]
    pub fn get_executor_name(&self) -> &str {
        &self.executor_name
    }

    /// 返回循环体执行项名称。
    ///
    /// 这是既有 Rust API，委托 Java 命名入口读取同一字段。
    #[must_use]
    pub fn executor_name(&self) -> &str {
        self.get_executor_name()
    }

    /// 返回子项是否执行成功。
    ///
    /// 对应 Java: `LoopFutureObj#isSuccess`。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// 设置失败异常。
    ///
    /// 对应 Java: `LoopFutureObj#setEx`。
    pub fn set_ex(&mut self, ex: Option<LiteflowError>) {
        self.ex = ex;
    }

    /// 设置循环体执行项名称。
    ///
    /// 对应 Java: `LoopFutureObj#setExecutorName`。
    pub fn set_executor_name(&mut self, executor_name: impl Into<String>) {
        self.executor_name = executor_name.into();
    }

    /// 设置子项成功状态。
    ///
    /// 对应 Java: `LoopFutureObj#setSuccess`。
    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }
}
