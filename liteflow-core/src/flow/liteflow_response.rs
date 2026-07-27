//! 链路执行结果封装。
//!
//! 对应 Java: `com.yomahub.liteflow.flow.LiteflowResponse`。

use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::Slot;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// 按节点首次出现顺序分组的执行步骤。
///
/// Java 使用 `LinkedHashMap<String, List<CmpStep>>`；Rust 用有序 `Vec` 保留同样
/// 的插入顺序，并借用响应中的步骤以避免复制。
pub type StepGroups<'a> = Vec<(&'a str, Vec<&'a CmpStep>)>;

/// LiteFlow 链路执行结果。
///
/// 对象保留执行 Slot 的共享引用以及创建响应时的步骤快照。与旧实现不同，创建
/// 响应不会清空 Slot 的步骤队列，因此 `get_slot()`、子链合并和诊断 API 可以
/// 继续观察同一份执行历史。
/// 对应 Java: `com.yomahub.liteflow.flow.LiteflowResponse`。
pub struct LiteflowResponse {
    pub request_id: String,
    pub chain_id: String,
    pub success: bool,
    /// LiteFlow 业务错误码；非 LiteFlow 错误或成功响应为 `None`。
    pub code: Option<String>,
    pub message: String,
    pub cause: Option<String>,
    pub steps: Vec<CmpStep>,
    /// 失败补偿步骤，顺序即实际回滚顺序。
    pub rollback_steps: Vec<CmpStep>,
    slot: Arc<Slot>,
}

impl LiteflowResponse {
    /// 从 Slot 和显式结果参数创建响应快照。
    ///
    /// 这是 Rust 执行主干的内部构造入口；公开 Java 对等入口是
    /// `new_main_response` 与 `new_inner_response`。
    pub(crate) fn new(
        slot: Arc<Slot>,
        success: bool,
        message: String,
        cause: Option<String>,
    ) -> Self {
        let steps = slot
            .steps
            .lock()
            .map(|steps| steps.clone())
            .unwrap_or_default();
        let rollback_steps = slot
            .rollback_steps
            .lock()
            .map(|steps| steps.clone())
            .unwrap_or_default();
        Self {
            request_id: slot.request_id.clone(),
            chain_id: slot.chain_id.clone(),
            success,
            code: None,
            message,
            cause,
            steps,
            rollback_steps,
            slot,
        }
    }

    /// 由主链 Slot 创建执行响应。
    ///
    /// Slot 不含异常时响应成功；含异常时失败，并把异常文本写入 message/cause。
    /// 参数 `slot` 对应 Java `slot`。对应 Java:
    /// `LiteflowResponse#newMainResponse(Slot)`。
    #[must_use]
    pub fn new_main_response(slot: Arc<Slot>) -> Self {
        let exception = slot.get_exception();
        Self::new_response(slot, exception)
    }

    /// 由执行前异常创建主链响应。
    ///
    /// Rust 没有 Java 方法重载，因此异常重载采用 `_with_cause` 后缀。
    /// 对应 Java: `LiteflowResponse#newMainResponse(Exception)`。
    #[must_use]
    pub fn new_main_response_with_cause(cause: impl Into<String>) -> Self {
        let cause = cause.into();
        let slot = Arc::new(Slot::new(String::new(), String::new(), Value::Null));
        Self::new_response(slot, Some(cause))
    }

    /// 由指定子链在共享 Slot 中记录的异常创建响应。
    ///
    /// 参数 `chain_id` 只用于选择子链异常；响应 chainId 仍取 Slot 当前主链 ID，
    /// 与 Java 行为一致。对应 Java:
    /// `LiteflowResponse#newInnerResponse(String, Slot)`。
    #[must_use]
    pub fn new_inner_response(chain_id: &str, slot: Arc<Slot>) -> Self {
        let exception = slot.get_sub_exception(chain_id);
        Self::new_response(slot, exception)
    }

    fn new_response(slot: Arc<Slot>, exception: Option<String>) -> Self {
        match exception {
            Some(exception) => Self::new(slot, false, exception.clone(), Some(exception)),
            None => Self::new(slot, true, String::new(), None),
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

    /// 设置执行成功状态。对应 Java: `LiteflowResponse#setSuccess`。
    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }

    /// 返回响应消息。对应 Java: `LiteflowResponse#getMessage`。
    #[must_use]
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// 设置响应消息。对应 Java: `LiteflowResponse#setMessage`。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// 返回业务错误码。对应 Java: `LiteflowResponse#getCode`。
    #[must_use]
    pub fn get_code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// 设置业务错误码。对应 Java: `LiteflowResponse#setCode`。
    pub fn set_code(&mut self, code: Option<String>) {
        self.code = code;
    }

    /// 返回失败原因文本。对应 Java: `LiteflowResponse#getCause`。
    #[must_use]
    pub fn get_cause(&self) -> Option<&str> {
        self.cause.as_deref()
    }

    /// 设置失败原因文本。对应 Java: `LiteflowResponse#setCause`。
    pub fn set_cause(&mut self, cause: Option<String>) {
        self.cause = cause;
    }

    /// 返回本次执行 Slot。对应 Java: `LiteflowResponse#getSlot`。
    #[must_use]
    pub fn get_slot(&self) -> &Arc<Slot> {
        &self.slot
    }

    /// 替换执行 Slot，并刷新公开的请求、链路和步骤快照。
    ///
    /// 对应 Java: `LiteflowResponse#setSlot`。Rust 同时刷新派生字段，避免响应公开
    /// 字段与新 Slot 不一致。
    pub fn set_slot(&mut self, slot: Arc<Slot>) {
        self.request_id = slot.request_id.clone();
        self.chain_id = slot.chain_id.clone();
        self.steps = slot
            .steps
            .lock()
            .map(|steps| steps.clone())
            .unwrap_or_default();
        self.rollback_steps = slot
            .rollback_steps
            .lock()
            .map(|steps| steps.clone())
            .unwrap_or_default();
        self.slot = slot;
    }

    /// 返回第一个上下文 Bean。
    ///
    /// 泛型 `T` 必须与 Java contextBeanArray 的首项类型一致。对应 Java:
    /// `LiteflowResponse#getFirstContextBean`。
    pub fn get_first_context_bean<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.slot.get_first_context_bean()
    }

    /// 按名称返回上下文 Bean。
    ///
    /// 参数 `context_name` 对应 Java `contextName`。对应 Java:
    /// `LiteflowResponse#getContextBean(String)`。
    pub fn get_context_bean<T: Any + Send + Sync>(&self, context_name: &str) -> Option<Arc<T>> {
        self.slot.get_context_bean(context_name)
    }

    /// 按运行时类型返回上下文 Bean。
    ///
    /// 对应 Java: `LiteflowResponse#getContextBean(Class)`。
    pub fn get_context_bean_by_type<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.slot.get_context_bean_by_type()
    }

    /// 按 nodeId 分组返回执行步骤，并保留首次出现顺序。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteSteps`。
    #[must_use]
    pub fn get_execute_steps(&self) -> StepGroups<'_> {
        group_steps(&self.steps)
    }

    /// 返回按真实执行顺序排列的步骤队列快照。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteStepQueue`。
    #[must_use]
    pub fn get_execute_step_queue(&self) -> &[CmpStep] {
        &self.steps
    }

    /// 返回按真实回滚顺序排列的步骤队列快照。
    ///
    /// 对应 Java: `LiteflowResponse#getRollbackStepQueue`。
    #[must_use]
    pub fn get_rollback_step_queue(&self) -> &[CmpStep] {
        &self.rollback_steps
    }

    /// 按 nodeId 分组返回回滚步骤，并保留首次出现顺序。
    ///
    /// 对应 Java: `LiteflowResponse#getRollbackSteps`。
    #[must_use]
    pub fn get_rollback_steps(&self) -> StepGroups<'_> {
        group_steps(&self.rollback_steps)
    }

    /// 返回不含耗时的执行步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteStepStr`。
    #[must_use]
    pub fn step_str(&self) -> String {
        self.steps
            .iter()
            .map(CmpStep::build_string)
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 返回不含耗时的执行步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteStepStr`。
    #[must_use]
    pub fn get_execute_step_str(&self) -> String {
        self.get_execute_step_str_without_time()
    }

    /// 返回带节点实例编号的执行步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteStepStrWithInstanceId`。
    #[must_use]
    pub fn get_execute_step_str_with_instance_id(&self) -> String {
        self.steps
            .iter()
            .map(CmpStep::build_string_with_instance_id)
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 返回包含执行耗时的步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteStepStrWithTime`。
    #[must_use]
    pub fn step_str_with_time(&self) -> String {
        self.steps
            .iter()
            .map(CmpStep::build_string_with_time)
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 返回包含执行耗时的步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteStepStrWithTime`。
    #[must_use]
    pub fn get_execute_step_str_with_time(&self) -> String {
        self.step_str_with_time()
    }

    /// 返回不含执行耗时的步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getExecuteStepStrWithoutTime`。
    #[must_use]
    pub fn get_execute_step_str_without_time(&self) -> String {
        self.step_str()
    }

    /// 返回不含耗时的回滚步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getRollbackStepStr`。
    #[must_use]
    pub fn rollback_step_str(&self) -> String {
        self.rollback_steps
            .iter()
            .map(CmpStep::build_string)
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 返回不含耗时的回滚步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getRollbackStepStr`。
    #[must_use]
    pub fn get_rollback_step_str(&self) -> String {
        self.get_rollback_step_str_without_time()
    }

    /// 返回包含回滚耗时的步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getRollbackStepStrWithTime`。
    #[must_use]
    pub fn get_rollback_step_str_with_time(&self) -> String {
        self.rollback_steps
            .iter()
            .map(CmpStep::build_rollback_string_with_time)
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 返回不含回滚耗时的步骤文本。
    ///
    /// 对应 Java: `LiteflowResponse#getRollbackStepStrWithoutTime`。
    #[must_use]
    pub fn get_rollback_step_str_without_time(&self) -> String {
        self.rollback_step_str()
    }

    /// 返回请求 ID。对应 Java: `LiteflowResponse#getRequestId`。
    #[must_use]
    pub fn get_request_id(&self) -> &str {
        self.slot.get_request_id()
    }

    /// 返回会话 ID。对应 Java: `LiteflowResponse#getConversationId`。
    #[must_use]
    pub fn get_conversation_id(&self) -> Option<&str> {
        self.slot.get_conversation_id()
    }

    /// 返回链路 ID。对应 Java: `LiteflowResponse#getChainId`。
    #[must_use]
    pub fn get_chain_id(&self) -> &str {
        &self.chain_id
    }

    /// 设置链路 ID。对应 Java: `LiteflowResponse#setChainId`。
    pub fn set_chain_id(&mut self, chain_id: impl Into<String>) {
        self.chain_id = chain_id.into();
    }

    /// 返回 WHEN 并行执行中发生超时的执行项。
    ///
    /// 对应 Java: `LiteflowResponse#getTimeoutItems`。
    #[must_use]
    pub fn get_timeout_items(&self) -> Vec<String> {
        self.slot.get_timeout_item_list()
    }

    /// 旧版具名上下文 Bean 访问入口。
    ///
    /// 保留现有 Rust API，并委托 Java 对等方法 `get_context_bean`。
    pub fn bean<T: Any + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        self.get_context_bean(name)
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
            .field("code", &self.code)
            .field("message", &self.message)
            .field("cause", &self.cause)
            .finish()
    }
}

fn group_steps(steps: &[CmpStep]) -> StepGroups<'_> {
    let mut groups: StepGroups<'_> = Vec::new();
    for step in steps {
        if let Some((_, grouped_steps)) = groups
            .iter_mut()
            .find(|(node_id, _)| *node_id == step.node_id.as_str())
        {
            grouped_steps.push(step);
        } else {
            groups.push((&step.node_id, vec![step]));
        }
    }
    groups
}
