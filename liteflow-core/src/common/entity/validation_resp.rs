//! 校验结果封装。
//!
//! 对应 Java: `com.yomahub.liteflow.common.entity.ValidationResp`。

use crate::exception::LiteflowError;

/// 表达脚本或规则校验是否成功，以及失败时保留的原始异常。
#[derive(Debug, Clone)]
pub struct ValidationResp {
    success: bool,
    cause: Option<LiteflowError>,
}

impl ValidationResp {
    /// 创建校验结果。对应 Java `ValidationResp(boolean, Exception)`。
    pub fn new(success: bool, cause: Option<LiteflowError>) -> Self {
        Self { success, cause }
    }

    /// 创建成功结果；成功时 cause 必须为空。
    pub fn success() -> Self {
        Self::new(true, None)
    }

    /// 创建失败结果并保留异常。
    pub fn fail(cause: LiteflowError) -> Self {
        Self::new(false, Some(cause))
    }

    /// 返回校验是否成功。对应 Java `isSuccess()`。
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// 修改成功标记。对应 Java `setSuccess(boolean)`。
    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }

    /// 返回失败原因；成功结果返回 None。对应 Java `getCause()`。
    pub fn cause(&self) -> Option<&LiteflowError> {
        self.cause.as_ref()
    }

    /// 返回失败时保留的异常。
    ///
    /// - 返回：失败结果返回原始 `LiteflowError` 引用，成功结果返回 `None`。
    ///
    /// 对应 Java: `ValidationResp#getCause`。
    #[must_use]
    pub fn get_cause(&self) -> Option<&LiteflowError> {
        self.cause()
    }

    /// 修改失败原因。对应 Java `setCause(Exception)`。
    pub fn set_cause(&mut self, cause: Option<LiteflowError>) {
        self.cause = cause;
    }
}
