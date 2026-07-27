//! 一次链路执行的共享状态。
//!
//! 对应 Java: `com.yomahub.liteflow.slot.Slot`。

use crate::core::NodeComponent;
use crate::el::NodeRef;
use crate::flow::element::chain::Chain;
use crate::flow::element::condition::Condition;
use crate::flow::entity::cmp_step::CmpStep;
use crate::flow::id::IdGeneratorHolder;
use crate::log::LFLoggerManager;
use crate::slot::Frame;
use dashmap::DashMap;
use serde_json::Value;
use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

const NODE_INPUT_PREFIX: &str = "_input_";
const NODE_OUTPUT_PREFIX: &str = "_output_";

/// 一次流程执行的共享数据槽。
///
/// Java 使用 `ConcurrentHashMap<String, Object>` 保存异构元数据；Rust 将固定职责
/// 拆成强类型字段，并使用 `serde_json::Value` 保存仍需动态表达的业务数据。
/// 对应 Java: `com.yomahub.liteflow.slot.Slot`。
pub struct Slot {
    pub request_id: String,
    pub chain_id: String,
    /// conversationId（2.15+：业务会话标识，ReAct Agent 连续对话场景）
    pub conversation_id: Option<String>,
    /// contextBeanMap
    pub beans: DashMap<String, Arc<dyn Any + Send + Sync>>,
    /// contextBean 的插入顺序，用于实现 Java `getFirstContextBean()`。
    context_bean_order: Mutex<Vec<String>>,
    /// requestData
    pub input: Mutex<Value>,
    /// 链路内共享数据
    pub data: DashMap<String, Value>,
    /// 主流程响应数据。
    response_data: Mutex<Option<Value>>,
    /// 子链请求数据，key 为 chainId。
    chain_request_data: DashMap<String, Value>,
    /// 同一子链多次调用时的请求数据队列。
    chain_request_queues: DashMap<String, Arc<Mutex<VecDeque<Value>>>>,
    /// 节点私有传递队列。
    private_delivery_queues: DashMap<String, Arc<Mutex<VecDeque<Value>>>>,
    /// 已进入当前 Slot 的 Chain 实例。
    chain_instances: DashMap<String, Arc<Chain>>,
    /// 决策路由结果。
    route_result: Mutex<Option<bool>>,
    /// executeSteps
    pub steps: Mutex<Vec<CmpStep>>,
    /// rollbackSteps（对应 Java Slot#getRollbackSteps）
    pub rollback_steps: Mutex<Vec<CmpStep>>,
    /// 已进入执行阶段且声明支持回滚的组件。
    ///
    /// 元组依次为 NodeInstanceId、组件实例、NodeRef 与分支 Frame。这里不保存
    /// 含 `Arc<Slot>` 的完整 CmpContext，避免 Slot 自身形成强引用环。发生普通异常
    /// 或超时时由 FlowExecutor 逆序消费；同一 NodeInstanceId 因重试产生多条记录
    /// 时只回滚一次，对齐 Java `NodeComponent#doRollback` 的去重语义。
    pub(crate) rollback_items: Mutex<Vec<(String, Arc<dyn NodeComponent>, NodeRef, Frame)>>,
    /// slot.exception
    pub exception: Mutex<Option<String>>,
    /// 子链异常，key 为子链 chainId。
    sub_exceptions: DashMap<String, String>,
    /// WHEN 并行执行中发生超时的执行项。
    timeout_items: Mutex<Vec<String>>,
    /// isEnd
    pub ended: AtomicBool,
    /// attachment（2.15+：Slot.setAttachment/getAttachment/hasAttachment/removeAttachment）
    pub attachments: DashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl Slot {
    pub fn new(request_id: String, chain_id: impl Into<String>, input: Value) -> Self {
        Self {
            request_id,
            chain_id: chain_id.into(),
            conversation_id: None,
            beans: DashMap::new(),
            context_bean_order: Mutex::new(Vec::new()),
            input: Mutex::new(input),
            data: DashMap::new(),
            response_data: Mutex::new(None),
            chain_request_data: DashMap::new(),
            chain_request_queues: DashMap::new(),
            private_delivery_queues: DashMap::new(),
            chain_instances: DashMap::new(),
            route_result: Mutex::new(None),
            steps: Mutex::new(Vec::new()),
            rollback_steps: Mutex::new(Vec::new()),
            rollback_items: Mutex::new(Vec::new()),
            exception: Mutex::new(None),
            sub_exceptions: DashMap::new(),
            timeout_items: Mutex::new(Vec::new()),
            ended: AtomicBool::new(false),
            attachments: DashMap::new(),
        }
    }

    /// 返回指定节点的输入数据快照。
    ///
    /// 参数 `node_id` 对应 Java 同名参数；未设置时返回 `None`。
    /// 对应 Java: `Slot#getInput`。
    #[must_use]
    pub fn get_input(&self, node_id: &str) -> Option<Value> {
        self.data
            .get(&format!("{NODE_INPUT_PREFIX}{node_id}"))
            .map(|value| value.clone())
    }

    /// 返回指定节点的输出数据快照。
    ///
    /// 参数 `node_id` 对应 Java 同名参数；未设置时返回 `None`。
    /// 对应 Java: `Slot#getOutput`。
    #[must_use]
    pub fn get_output(&self, node_id: &str) -> Option<Value> {
        self.data
            .get(&format!("{NODE_OUTPUT_PREFIX}{node_id}"))
            .map(|value| value.clone())
    }

    /// 设置指定节点的输入数据。
    ///
    /// 参数 `node_id`、`input` 分别对应 Java `nodeId`、`t`。
    /// 对应 Java: `Slot#setInput`。
    pub fn set_input(&self, node_id: impl AsRef<str>, input: Value) {
        self.data
            .insert(format!("{NODE_INPUT_PREFIX}{}", node_id.as_ref()), input);
    }

    /// 设置指定节点的输出数据。
    ///
    /// 参数 `node_id`、`output` 分别对应 Java `nodeId`、`t`。
    /// 对应 Java: `Slot#setOutput`。
    pub fn set_output(&self, node_id: impl AsRef<str>, output: Value) {
        self.data
            .insert(format!("{NODE_OUTPUT_PREFIX}{}", node_id.as_ref()), output);
    }

    /// 返回流程响应数据快照。对应 Java: `Slot#getResponseData`。
    #[must_use]
    pub fn get_response_data(&self) -> Option<Value> {
        self.response_data
            .lock()
            .ok()
            .and_then(|response| response.clone())
    }

    /// 设置流程响应数据。
    ///
    /// 参数 `response_data` 对应 Java 参数 `t`。对应 Java: `Slot#setResponseData`。
    pub fn set_response_data(&self, response_data: Value) {
        if let Ok(mut current) = self.response_data.lock() {
            *current = Some(response_data);
        }
    }

    /// 返回指定子链的请求数据快照。
    ///
    /// 参数 `chain_id` 对应 Java 同名参数。对应 Java: `Slot#getChainReqData`。
    #[must_use]
    pub fn get_chain_req_data(&self, chain_id: &str) -> Option<Value> {
        self.chain_request_data
            .get(chain_id)
            .map(|value| value.clone())
    }

    /// 设置指定子链的请求数据。
    ///
    /// 参数 `chain_id`、`request_data` 对应 Java `chainId`、`t`。
    /// 对应 Java: `Slot#setChainReqData`。
    pub fn set_chain_req_data(&self, chain_id: impl Into<String>, request_data: Value) {
        self.chain_request_data
            .insert(chain_id.into(), request_data);
    }

    /// 从指定子链的请求队列头部取出一个值。
    ///
    /// 队列不存在或为空时返回 `None`。对应 Java: `Slot#getChainReqDataFromQueue`。
    pub fn get_chain_req_data_from_queue(&self, chain_id: &str) -> Option<Value> {
        self.chain_request_queues
            .get(chain_id)
            .and_then(|queue| queue.lock().ok()?.pop_front())
    }

    /// 向指定子链的请求队列尾部追加一个值。
    ///
    /// 参数 `chain_id`、`request_data` 对应 Java `chainId`、`t`。
    /// 对应 Java: `Slot#setChainReqData2Queue`。
    pub fn set_chain_req_data2_queue(&self, chain_id: impl Into<String>, request_data: Value) {
        let chain_id = chain_id.into();
        let queue = self
            .chain_request_queues
            .entry(chain_id)
            .or_insert_with(|| Arc::new(Mutex::new(VecDeque::new())))
            .clone();
        if let Ok(mut queue) = queue.lock() {
            queue.push_back(request_data);
        }
    }

    /// 向指定节点的私有传递队列追加数据。
    ///
    /// 参数 `node_id`、`delivery_data` 对应 Java `nodeId`、`t`。
    /// 对应 Java: `Slot#setPrivateDeliveryData`。
    pub fn set_private_delivery_data(&self, node_id: impl Into<String>, delivery_data: Value) {
        let node_id = node_id.into();
        let queue = self
            .private_delivery_queues
            .entry(node_id)
            .or_insert_with(|| Arc::new(Mutex::new(VecDeque::new())))
            .clone();
        if let Ok(mut queue) = queue.lock() {
            queue.push_back(delivery_data);
        }
    }

    /// 返回指定节点的私有传递队列快照。
    ///
    /// 参数 `node_id` 对应 Java 同名参数。对应 Java: `Slot#getPrivateDeliveryQueue`。
    #[must_use]
    pub fn get_private_delivery_queue(&self, node_id: &str) -> Option<VecDeque<Value>> {
        self.private_delivery_queues
            .get(node_id)
            .and_then(|queue| queue.lock().ok().map(|queue| queue.clone()))
    }

    /// 从指定节点的私有传递队列头部取出一个值。
    ///
    /// 参数 `node_id` 对应 Java 同名参数。对应 Java: `Slot#getPrivateDeliveryData`。
    pub fn get_private_delivery_data(&self, node_id: &str) -> Option<Value> {
        self.private_delivery_queues
            .get(node_id)
            .and_then(|queue| queue.lock().ok()?.pop_front())
    }

    /// 登记当前 Slot 中使用的 Chain 实例。
    ///
    /// 参数 `chain` 对应 Java 同名参数；相同 ID 只保留首次实例。
    /// 对应 Java: `Slot#addChainInstance`。
    pub fn add_chain_instance(&self, chain: Arc<Chain>) {
        self.chain_instances
            .entry(chain.id.clone())
            .or_insert(chain);
    }

    /// 返回指定 ID 的当前 Chain 实例。
    ///
    /// 参数 `current_chain_id` 对应 Java 同名参数。
    /// 对应 Java: `Slot#getCurrentChainInstance`。
    #[must_use]
    pub fn get_current_chain_instance(&self, current_chain_id: &str) -> Option<Arc<Chain>> {
        self.chain_instances
            .get(current_chain_id)
            .map(|chain| chain.clone())
    }

    /// 返回当前路由结果。对应 Java: `Slot#getRouteResult`。
    #[must_use]
    pub fn get_route_result(&self) -> Option<bool> {
        self.route_result.lock().ok().and_then(|result| *result)
    }

    /// 设置当前路由结果。
    ///
    /// 参数 `route_result` 对应 Java 同名参数。对应 Java: `Slot#setRouteResult`。
    pub fn set_route_result(&self, route_result: bool) {
        if let Ok(mut current) = self.route_result.lock() {
            *current = Some(route_result);
        }
    }

    /// 保存当前任务的 SWITCH 结果。
    ///
    /// Java 通过线程 ID 拼接元数据键；Rust 用 `frame` 显式承载任务隔离状态。
    /// 参数 `key`、`switch_result` 对应 Java `key`、`t`。
    /// 对应 Java: `Slot#setSwitchResult`。
    pub fn set_switch_result(
        &self,
        frame: &mut Frame,
        key: impl Into<String>,
        switch_result: Value,
    ) {
        frame.set_switch_result(key.into(), switch_result);
    }

    /// 返回当前任务的 SWITCH 结果。
    ///
    /// 参数 `frame` 是 Java ThreadLocal 的 Rust 映射。
    /// 对应 Java: `Slot#getSwitchResult`。
    #[must_use]
    pub fn get_switch_result(&self, frame: &Frame, key: &str) -> Option<Value> {
        frame.get_switch_result(key)
    }

    /// 保存当前任务的 IF 结果。对应 Java: `Slot#setIfResult`。
    pub fn set_if_result(&self, frame: &mut Frame, key: impl Into<String>, result: bool) {
        frame.set_if_result(key.into(), result);
    }

    /// 返回当前任务的 IF 结果。对应 Java: `Slot#getIfResult`。
    #[must_use]
    pub fn get_if_result(&self, frame: &Frame, key: &str) -> Option<bool> {
        frame.get_if_result(key)
    }

    /// 保存当前任务的 AND/OR 结果。对应 Java: `Slot#setAndOrResult`。
    pub fn set_and_or_result(&self, frame: &mut Frame, key: impl Into<String>, result: bool) {
        frame.set_and_or_result(key.into(), result);
    }

    /// 返回当前任务的 AND/OR 结果。对应 Java: `Slot#getAndOrResult`。
    #[must_use]
    pub fn get_and_or_result(&self, frame: &Frame, key: &str) -> Option<bool> {
        frame.get_and_or_result(key)
    }

    /// 保存当前任务的 NOT 结果。对应 Java: `Slot#setNotResult`。
    pub fn set_not_result(&self, frame: &mut Frame, key: impl Into<String>, result: bool) {
        frame.set_not_result(key.into(), result);
    }

    /// 返回当前任务的 NOT 结果。对应 Java: `Slot#getNotResult`。
    #[must_use]
    pub fn get_not_result(&self, frame: &Frame, key: &str) -> Option<bool> {
        frame.get_not_result(key)
    }

    /// 保存当前任务的 FOR 次数。对应 Java: `Slot#setForResult`。
    pub fn set_for_result(&self, frame: &mut Frame, key: impl Into<String>, for_count: usize) {
        frame.set_for_result(key.into(), for_count);
    }

    /// 返回当前任务的 FOR 次数。对应 Java: `Slot#getForResult`。
    #[must_use]
    pub fn get_for_result(&self, frame: &Frame, key: &str) -> Option<usize> {
        frame.get_for_result(key)
    }

    /// 保存当前任务的 WHILE 结果。对应 Java: `Slot#setWhileResult`。
    pub fn set_while_result(&self, frame: &mut Frame, key: impl Into<String>, while_flag: bool) {
        frame.set_while_result(key.into(), while_flag);
    }

    /// 返回当前任务的 WHILE 结果。对应 Java: `Slot#getWhileResult`。
    #[must_use]
    pub fn get_while_result(&self, frame: &Frame, key: &str) -> Option<bool> {
        frame.get_while_result(key)
    }

    /// 保存当前任务的 BREAK 结果。对应 Java: `Slot#setBreakResult`。
    pub fn set_break_result(&self, frame: &mut Frame, key: impl Into<String>, break_flag: bool) {
        frame.set_break_result(key.into(), break_flag);
    }

    /// 返回当前任务的 BREAK 结果。对应 Java: `Slot#getBreakResult`。
    #[must_use]
    pub fn get_break_result(&self, frame: &Frame, key: &str) -> Option<bool> {
        frame.get_break_result(key)
    }

    /// 保存当前任务的迭代结果。
    ///
    /// Java 保存 `Iterator<?>`；Rust 保存可深复制的 `Value` 队列，使并行 Frame
    /// 克隆后各自拥有独立游标。对应 Java: `Slot#setIteratorResult`。
    pub fn set_iterator_result(
        &self,
        frame: &mut Frame,
        key: impl Into<String>,
        iterator: impl IntoIterator<Item = Value>,
    ) {
        frame.set_iterator_result(key.into(), iterator);
    }

    /// 返回当前任务的迭代结果快照。
    ///
    /// 对应 Java: `Slot#getIteratorResult`。
    #[must_use]
    pub fn get_iterator_result(
        &self,
        frame: &Frame,
        key: &str,
    ) -> Option<std::collections::vec_deque::IntoIter<Value>> {
        frame.get_iterator_result(key).map(VecDeque::into_iter)
    }

    /// 将 Condition 压入当前任务调用栈。
    ///
    /// 参数 `condition` 对应 Java 同名参数。对应 Java: `Slot#pushCondition`。
    pub fn push_condition(&self, frame: &Frame, condition: Arc<dyn Condition>) {
        frame.push_condition(condition);
    }

    /// 弹出当前任务调用栈顶的 Condition。
    ///
    /// 空栈返回 `None`，避免 Java `Deque#pop` 的空栈异常。
    /// 对应 Java: `Slot#popCondition`。
    pub fn pop_condition(&self, frame: &Frame) -> Option<Arc<dyn Condition>> {
        frame.pop_condition()
    }

    /// 返回当前任务调用栈顶的 Condition。
    ///
    /// 对应 Java: `Slot#getCurrentCondition`。
    #[must_use]
    pub fn get_current_condition(&self, frame: &Frame) -> Option<Arc<dyn Condition>> {
        frame.current_condition()
    }

    /// 返回当前任务的 Condition 调用栈快照。
    ///
    /// 供运行时从内向外查找 bind 数据。对应 Java: `Slot#getConditionStack`。
    #[must_use]
    pub fn get_condition_stack(&self, frame: &Frame) -> Vec<Arc<dyn Condition>> {
        frame.condition_stack()
    }

    /// 添加具名上下文 Bean，并保留 Java 参数数组的插入顺序。
    ///
    /// 对应 Java 构造 Slot 时写入 `contextBeanList` 的逻辑。
    pub fn insert_context_bean(
        &self,
        context_name: impl Into<String>,
        context_bean: Arc<dyn Any + Send + Sync>,
    ) {
        let context_name = context_name.into();
        let is_new = !self.beans.contains_key(&context_name);
        self.beans.insert(context_name.clone(), context_bean);
        if is_new {
            if let Ok(mut order) = self.context_bean_order.lock() {
                order.push(context_name);
            }
        }
    }

    /// 按名称取得上下文 Bean。
    ///
    /// 参数 `context_name` 对应 Java `contextBeanKey`；不存在或类型不符时返回
    /// `None`，由 Rust `Option` 表达 Java 的异常边界。
    /// 对应 Java: `Slot#getContextBean(String)`。
    pub fn get_context_bean<T: Any + Send + Sync>(&self, context_name: &str) -> Option<Arc<T>> {
        self.beans
            .get(context_name)
            .and_then(|bean| bean.clone().downcast::<T>().ok())
    }

    /// 按 Rust 运行时类型取得第一个匹配的上下文 Bean。
    ///
    /// 对应 Java: `Slot#getContextBean(Class)`。
    pub fn get_context_bean_by_type<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let order = self.context_bean_order.lock().ok()?.clone();
        order
            .iter()
            .find_map(|context_name| self.get_context_bean::<T>(context_name))
            .or_else(|| {
                self.beans
                    .iter()
                    .find_map(|entry| entry.value().clone().downcast::<T>().ok())
            })
    }

    /// 返回插入顺序中的第一个上下文 Bean。
    ///
    /// 泛型 `T` 必须与首个 Bean 的实际类型一致。对应 Java:
    /// `Slot#getFirstContextBean`。
    pub fn get_first_context_bean<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let context_name = self.context_bean_order.lock().ok()?.first()?.clone();
        self.get_context_bean(&context_name)
    }

    /// 返回按插入顺序排列的上下文 Bean 快照。
    ///
    /// Rust 使用 `(名称, Arc<dyn Any>)` 映射 Java `Tuple`。
    /// 对应 Java: `Slot#getContextBeanList`。
    #[must_use]
    pub fn get_context_bean_list(&self) -> Vec<(String, Arc<dyn Any + Send + Sync>)> {
        let order = self
            .context_bean_order
            .lock()
            .map(|order| order.clone())
            .unwrap_or_default();
        order
            .into_iter()
            .filter_map(|context_name| {
                self.beans
                    .get(&context_name)
                    .map(|bean| (context_name, bean.clone()))
            })
            .collect()
    }

    /// 返回请求 ID。对应 Java: `Slot#getRequestId`。
    #[must_use]
    pub fn get_request_id(&self) -> &str {
        &self.request_id
    }

    /// 使用已注册的 RequestIdGenerator 生成并保存请求 ID。
    ///
    /// 对应 Java: `Slot#generateRequestId`。
    pub fn generate_request_id(&mut self) {
        self.request_id = IdGeneratorHolder::generate();
    }

    /// 保存调用方提供的请求 ID。
    ///
    /// 参数 `request_id` 对应 Java 同名参数。对应 Java: `Slot#putRequestId`。
    pub fn put_request_id(&mut self, request_id: impl Into<String>) {
        self.request_id = request_id.into();
    }

    /// 返回当前主链 ID。对应 Java: `Slot#getChainId`。
    #[must_use]
    pub fn get_chain_id(&self) -> &str {
        &self.chain_id
    }

    /// 设置当前主链 ID。
    ///
    /// 与 Java 一样，已经存在非空 Chain ID 时不覆盖。
    /// 参数 `chain_id` 对应 Java 同名参数。对应 Java: `Slot#setChainId`。
    pub fn set_chain_id(&mut self, chain_id: impl Into<String>) {
        if self.chain_id.is_empty() {
            self.chain_id = chain_id.into();
        }
    }

    /// 设置当前主链名称。
    ///
    /// 该方法是 `set_chain_id` 的废弃兼容入口。对应 Java: `Slot#setChainName`。
    #[deprecated(note = "使用 set_chain_id")]
    pub fn set_chain_name(&mut self, chain_name: impl Into<String>) {
        self.set_chain_id(chain_name);
    }

    /// 返回当前主链名称。
    ///
    /// 该方法是 `get_chain_id` 的废弃兼容入口。对应 Java: `Slot#getChainName`。
    #[deprecated(note = "使用 get_chain_id")]
    #[must_use]
    pub fn get_chain_name(&self) -> &str {
        self.get_chain_id()
    }

    /// 返回会话 ID。对应 Java: `Slot#getConversationId`。
    #[must_use]
    pub fn get_conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    /// 设置当前 Chain 执行的会话标识。
    ///
    /// ReAct Agent 等连续对话场景会在同一 Chain 内共享该值，使后续 Agent
    /// 复用 workspace，并按 nodeId 隔离各自记忆。
    /// 参数 `conversation_id` 对应 Java 同名参数。
    /// 对应 Java: `Slot#setConversationId`。
    pub fn set_conversation_id(&mut self, conversation_id: impl Into<String>) {
        self.conversation_id = Some(conversation_id.into());
    }

    /// 添加执行步骤。
    ///
    /// 参数 `step` 对应 Java 同名参数。对应 Java: `Slot#addStep`。
    pub fn add_step(&self, step: CmpStep) {
        if let Ok(mut steps) = self.steps.lock() {
            steps.push(step);
        }
    }

    /// 返回执行步骤快照。对应 Java: `Slot#getExecuteSteps`。
    #[must_use]
    pub fn get_execute_steps(&self) -> Vec<CmpStep> {
        self.steps
            .lock()
            .map(|steps| steps.clone())
            .unwrap_or_default()
    }

    /// 构建执行步骤字符串。
    ///
    /// 参数 `with_time_spent` 对应 Java 同名参数；步骤之间使用 `==>` 连接。
    /// 对应 Java: `Slot#getExecuteStepStr(boolean)`。
    #[must_use]
    pub fn get_execute_step_str(&self, with_time_spent: bool) -> String {
        self.get_execute_steps()
            .iter()
            .map(|step| {
                if with_time_spent {
                    step.build_string_with_time()
                } else {
                    step.build_string()
                }
            })
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 构建包含节点实例 ID 的执行步骤字符串。
    ///
    /// 对应 Java: `Slot#getExecuteStepStrWithInstanceId`。
    #[must_use]
    pub fn get_execute_step_str_with_instance_id(&self) -> String {
        self.get_execute_steps()
            .iter()
            .map(CmpStep::build_string_with_instance_id)
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 打印包含耗时的执行步骤。
    ///
    /// 对应 Java: `Slot#printStep`。
    pub fn print_step(&self) {
        LFLoggerManager::get_logger("liteflow_core::slot::Slot").info(&format!(
            "CHAIN_NAME[{}]\n{}",
            self.get_chain_id(),
            self.get_execute_step_str(true)
        ));
    }

    /// 添加回滚步骤。
    ///
    /// 参数 `step` 对应 Java 同名参数。对应 Java: `Slot#addRollbackStep`。
    pub fn add_rollback_step(&self, step: CmpStep) {
        if let Ok(mut steps) = self.rollback_steps.lock() {
            steps.push(step);
        }
    }

    /// 返回回滚步骤快照。对应 Java: `Slot#getRollbackSteps`。
    #[must_use]
    pub fn get_rollback_steps(&self) -> Vec<CmpStep> {
        self.rollback_steps
            .lock()
            .map(|steps| steps.clone())
            .unwrap_or_default()
    }

    /// 构建回滚步骤字符串。
    ///
    /// 参数 `with_rollback_time_spent` 对应 Java 同名参数。
    /// 对应 Java: `Slot#getRollbackStepStr(boolean)`。
    #[must_use]
    pub fn get_rollback_step_str(&self, with_rollback_time_spent: bool) -> String {
        self.get_rollback_steps()
            .iter()
            .map(|step| {
                if with_rollback_time_spent {
                    step.build_rollback_string_with_time()
                } else {
                    step.build_string()
                }
            })
            .collect::<Vec<_>>()
            .join("==>")
    }

    /// 打印包含耗时的回滚步骤。
    ///
    /// 对应 Java: `Slot#printRollbackStep`。
    pub fn print_rollback_step(&self) {
        LFLoggerManager::get_logger("liteflow_core::slot::Slot").info(&format!(
            "ROLLBACK_CHAIN_NAME[{}]\n{}",
            self.get_chain_id(),
            self.get_rollback_step_str(true)
        ));
    }

    /// 返回主链异常文本。对应 Java: `Slot#getException`。
    #[must_use]
    pub fn get_exception(&self) -> Option<String> {
        self.exception
            .lock()
            .ok()
            .and_then(|exception| exception.clone())
    }

    /// 设置主链异常文本。对应 Java: `Slot#setException`。
    pub fn set_exception(&self, exception: impl Into<String>) {
        if let Ok(mut current) = self.exception.lock() {
            *current = Some(exception.into());
        }
    }

    /// 删除主链异常。对应 Java: `Slot#removeException`。
    pub fn remove_exception(&self) {
        if let Ok(mut current) = self.exception.lock() {
            *current = None;
        }
    }

    /// 记录子链异常。对应 Java 保存 `SUB_EXCEPTION_PREFIX + chainId` 的元数据。
    pub fn set_sub_exception(&self, chain_id: impl Into<String>, exception: impl Into<String>) {
        self.sub_exceptions
            .insert(chain_id.into(), exception.into());
    }

    /// 返回指定子链异常。对应 Java: `Slot#getSubException`。
    #[must_use]
    pub fn get_sub_exception(&self, chain_id: &str) -> Option<String> {
        self.sub_exceptions
            .get(chain_id)
            .map(|exception| exception.clone())
    }

    /// 记录发生超时的执行项。对应 Java: `Slot#addTimeoutItem`。
    pub fn add_timeout_item(&self, executor_item: impl Into<String>) {
        if let Ok(mut timeout_items) = self.timeout_items.lock() {
            timeout_items.push(executor_item.into());
        }
    }

    /// 返回超时执行项快照。对应 Java: `Slot#getTimeoutItemList`。
    #[must_use]
    pub fn get_timeout_item_list(&self) -> Vec<String> {
        self.timeout_items
            .lock()
            .map(|timeout_items| timeout_items.clone())
            .unwrap_or_default()
    }

    /// setAttachment(key, value)
    pub fn set_attachment<T: Any + Send + Sync>(&self, key: impl Into<String>, value: T) {
        self.attachments.insert(key.into(), Arc::new(value));
    }
    /// getAttachment(key)
    pub fn get_attachment<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.attachments
            .get(key)
            .and_then(|v| v.clone().downcast::<T>().ok())
    }
    /// hasAttachment(key)
    pub fn has_attachment(&self, key: &str) -> bool {
        self.attachments.contains_key(key)
    }
    /// removeAttachment(key)
    pub fn remove_attachment(&self, key: &str) {
        self.attachments.remove(key);
    }
}
