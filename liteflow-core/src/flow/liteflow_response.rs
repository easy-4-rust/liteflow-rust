//! 对应 flow.LiteflowResponse。

use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::Slot;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

pub struct LiteflowResponse {
    pub request_id: String,
    pub chain_id: String,
    pub success: bool,
    pub message: String,
    pub cause: Option<String>,
    pub steps: Vec<CmpStep>,
    /// 失败补偿步骤，顺序即实际回滚顺序。
    pub rollback_steps: Vec<CmpStep>,
    slot: Arc<Slot>,
}

impl LiteflowResponse {
    pub(crate) fn new(
        slot: Arc<Slot>,
        success: bool,
        message: String,
        cause: Option<String>,
    ) -> Self {
        let steps = slot
            .steps
            .lock()
            .map(|mut s| std::mem::take(&mut *s))
            .unwrap_or_default();
        let rollback_steps = slot
            .rollback_steps
            .lock()
            .map(|mut steps| std::mem::take(&mut *steps))
            .unwrap_or_default();
        Self {
            request_id: slot.request_id.clone(),
            chain_id: slot.chain_id.clone(),
            success,
            message,
            cause,
            steps,
            rollback_steps,
            slot,
        }
    }

    /// 创建执行前规则初始化失败响应。
    ///
    /// 参数保留请求 id、chain id 和原始输入，使兼容的非 `Result` 执行入口仍能
    /// 返回完整诊断；推荐需要区分初始化错误的调用者使用 Vernal `try_execute`。
    #[must_use]
    pub fn initialization_failure(
        request_id: impl Into<String>,
        chain_id: impl Into<String>,
        input: Value,
        cause: impl Into<String>,
    ) -> Self {
        let cause = cause.into();
        Self::new(
            Arc::new(Slot::new(request_id.into(), chain_id, input)),
            false,
            "rule initialization failed".to_string(),
            Some(cause),
        )
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    /// getExecuteStepStr()：a[10ms]==>b[3ms]==>c[ex:xxx]
    pub fn step_str(&self) -> String {
        self.steps
            .iter()
            .map(|s| {
                let mut seg = format!("{}[{}ms]", s.node_id, s.time_spent_ms());
                if let Some(e) = &s.exception {
                    seg.push_str(&format!("[ex:{e}]"));
                }
                seg
            })
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// getExecuteStepStrWithTime()
    pub fn step_str_with_time(&self) -> String {
        self.step_str()
    }

    /// getRollbackStepStr()：按真实回滚顺序输出组件与回滚耗时。
    pub fn rollback_step_str(&self) -> String {
        self.rollback_steps
            .iter()
            .map(|step| {
                let mut segment = format!("{}[{}ms]", step.node_id, step.rollback_time_spent_ms());
                if let Some(error) = &step.exception {
                    segment.push_str(&format!("[ex:{error}]"));
                }
                segment
            })
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// getContextBean
    pub fn bean<T: Any + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        self.slot
            .beans
            .get(name)
            .and_then(|v| v.clone().downcast::<T>().ok())
    }

    pub fn data(&self, key: &str) -> Option<Value> {
        self.slot.data.get(key).map(|v| v.clone())
    }
    pub fn data_as<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.slot
            .data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// slot.exception
    pub fn slot_exception(&self) -> Option<String> {
        self.slot.exception.lock().ok().and_then(|e| e.clone())
    }
}

impl std::fmt::Debug for LiteflowResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiteflowResponse")
            .field("request_id", &self.request_id)
            .field("chain_id", &self.chain_id)
            .field("success", &self.success)
            .field("message", &self.message)
            .field("cause", &self.cause)
            .finish()
    }
}
