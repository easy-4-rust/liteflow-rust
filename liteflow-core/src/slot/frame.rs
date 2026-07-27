//! 节点执行路径帧。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

use crate::flow::element::condition::Condition;
use serde_json::Value;

/// 对应 Java Node 的 loopIndexTL/loopObjectTL 与 Slot.conditionStack。
#[derive(Default)]
pub struct Frame {
    /// `(loopIndex, loopObject)` 栈。
    pub loops: Vec<(usize, Option<Value>)>,
    /// 与 `loops` 同下标的 Java LoopCondition 身份；Rust 原生 push 路径为 None。
    loop_condition_keys: Vec<Option<usize>>,
    /// Condition 级 bind 键值栈。
    pub binds: Vec<(String, String)>,
    /// 当前 Chain 指定的执行器构建器名称。
    chain_thread_pool: Option<String>,
    /// 当前正在执行的 Chain ID；子链进入时覆盖父链快照。
    current_chain_id: Option<String>,
    /// 当前 Condition 指定的执行器构建器名称。
    condition_thread_pool: Option<String>,
    /// 当前 SWITCH 条件允许跳转的目标节点 ID。
    switch_target_list: Vec<String>,
    /// Java Slot 中按线程隔离的 SWITCH 求值结果。
    switch_results: HashMap<String, Value>,
    /// Java Slot 中按线程隔离的 IF 求值结果。
    if_results: HashMap<String, bool>,
    /// Java Slot 中按线程隔离的 AND/OR 求值结果。
    and_or_results: RwLock<HashMap<String, bool>>,
    /// Java Slot 中按线程隔离的 NOT 求值结果。
    not_results: RwLock<HashMap<String, bool>>,
    /// Java Slot 中按线程隔离的 FOR 次数结果。
    for_results: HashMap<String, usize>,
    /// Java Slot 中按线程隔离的 WHILE 求值结果。
    while_results: HashMap<String, bool>,
    /// Java Slot 中按线程隔离的 BREAK 求值结果。
    break_results: HashMap<String, bool>,
    /// Java Slot 中按线程隔离的 ITERATOR 结果。
    iterator_results: HashMap<String, VecDeque<Value>>,
    /// 当前任务的 Condition 调用栈。
    condition_stack: RwLock<Vec<Arc<dyn Condition>>>,
    /// Java Chain#runtimeIdTL 的任务隔离运行标识。
    runtime_id: Option<u64>,
    /// Node#accessResult 的任务隔离缓存。
    node_access_results: HashMap<String, bool>,
    /// Node#isContinueOnErrorResult 的任务隔离缓存。
    node_continue_on_error_results: RwLock<HashMap<String, bool>>,
    /// Node#stepData 的任务隔离数据。
    ///
    /// 组件在 `process` 中通过共享 `CmpContext` 写入，Node 在 finally 阶段读取。
    node_step_data: Arc<RwLock<HashMap<String, Value>>>,
    /// Node#getItemResultMetaValue 的单次执行结果缓存。
    ///
    /// 使用锁允许 `Executable::execute(&Frame)` 在不要求调用方持有可变引用的
    /// 情况下写入结果；自定义 Clone 会复制当前快照，子任务之间不会共享本锁。
    node_item_results: RwLock<HashMap<String, Value>>,
}

impl Clone for Frame {
    /// 深复制任务执行状态。
    ///
    /// Java 的 `TransmittableThreadLocal` 会在子任务创建时复制父任务值；这里除
    /// 不可变 Condition 对象使用 `Arc` 外，所有集合及节点结果缓存都复制快照，
    /// 避免并行分支互相覆盖结果。
    fn clone(&self) -> Self {
        let node_step_data = self
            .node_step_data
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let node_item_results = self
            .node_item_results
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        Self {
            loops: self.loops.clone(),
            loop_condition_keys: self.loop_condition_keys.clone(),
            binds: self.binds.clone(),
            chain_thread_pool: self.chain_thread_pool.clone(),
            current_chain_id: self.current_chain_id.clone(),
            condition_thread_pool: self.condition_thread_pool.clone(),
            switch_target_list: self.switch_target_list.clone(),
            switch_results: self.switch_results.clone(),
            if_results: self.if_results.clone(),
            and_or_results: RwLock::new(
                self.and_or_results
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            ),
            not_results: RwLock::new(
                self.not_results
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            ),
            for_results: self.for_results.clone(),
            while_results: self.while_results.clone(),
            break_results: self.break_results.clone(),
            iterator_results: self.iterator_results.clone(),
            condition_stack: RwLock::new(
                self.condition_stack
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            ),
            runtime_id: self.runtime_id,
            node_access_results: self.node_access_results.clone(),
            node_continue_on_error_results: RwLock::new(
                self.node_continue_on_error_results
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            ),
            node_step_data: Arc::new(RwLock::new(node_step_data)),
            node_item_results: RwLock::new(node_item_results),
        }
    }
}

impl Frame {
    /// 创建根执行帧。
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// 为同一次组件调用克隆上下文。
    ///
    /// 普通 `Clone` 深复制 stepData，供父子任务隔离；组件适配器为了跨 `await`
    /// 取得拥有所有权的 `CmpContext` 时，必须与 Node finally 阶段共享同一份
    /// stepData。对应 Java TransmittableThreadLocal 在同一任务内可见的语义。
    pub(crate) fn clone_for_component(&self) -> Self {
        let mut frame = self.clone();
        frame.node_step_data = self.node_step_data.clone();
        frame
    }

    /// 压入循环下标和循环对象。
    #[must_use]
    pub fn push(&self, index: usize, object: Option<Value>) -> Self {
        let mut frame = self.clone();
        frame.loops.push((index, object));
        frame.loop_condition_keys.push(None);
        frame
    }

    /// 压入 Condition 级绑定数据。
    #[must_use]
    pub fn push_bind(&self, pairs: &[(String, String)]) -> Self {
        if pairs.is_empty() {
            return self.clone();
        }
        let mut frame = self.clone();
        frame.binds.extend(pairs.iter().cloned());
        frame
    }

    /// 写入当前 Chain 的执行器构建器名称。
    ///
    /// 对应 Java `Chain#setThreadPoolExecutorClass` 在执行期间由
    /// `ExecutorConditionBuilder` 读取的链级配置。
    #[must_use]
    pub fn with_chain_thread_pool(&self, thread_pool: Option<&str>) -> Self {
        let mut frame = self.clone();
        frame.chain_thread_pool = thread_pool.map(ToOwned::to_owned);
        frame
    }

    /// 写入当前正在执行的 Chain ID。
    ///
    /// 子链基于父 Frame 克隆后覆盖该值，对应 Java `Node#getCurrChainId` 使用的
    /// Node 任务局部字段。
    #[must_use]
    pub fn with_current_chain_id(&self, current_chain_id: impl Into<String>) -> Self {
        let mut frame = self.clone();
        frame.current_chain_id = Some(current_chain_id.into());
        frame
    }

    /// 返回当前正在执行的 Chain ID。
    #[must_use]
    pub fn current_chain_id(&self) -> Option<&str> {
        self.current_chain_id.as_deref()
    }

    /// 写入当前 Condition 的执行器构建器名称。
    ///
    /// 对应 Java `LoopCondition#setThreadPoolExecutorClass`。
    #[must_use]
    pub fn with_condition_thread_pool(&self, thread_pool: Option<&str>) -> Self {
        let mut frame = self.clone();
        frame.condition_thread_pool = thread_pool.map(ToOwned::to_owned);
        frame
    }

    /// 写入当前 SWITCH 条件的目标节点 ID 列表。
    ///
    /// 参数 `target_list` 对应 Java `SwitchCondition#getTargetList` 中可执行对象的
    /// ID 投影；返回携带该条件上下文的新执行帧，供 `NodeSwitchComponent` 在路由
    /// 计算期间读取。
    #[must_use]
    pub fn with_switch_target_list(&self, target_list: &[String]) -> Self {
        let mut frame = self.clone();
        frame.switch_target_list = target_list.to_vec();
        frame
    }

    /// 返回当前 Chain 的执行器构建器名称。
    #[must_use]
    pub fn chain_thread_pool(&self) -> Option<&str> {
        self.chain_thread_pool.as_deref()
    }

    /// 返回当前 Condition 的执行器构建器名称。
    #[must_use]
    pub fn condition_thread_pool(&self) -> Option<&str> {
        self.condition_thread_pool.as_deref()
    }

    /// 返回当前 SWITCH 条件允许跳转的目标节点 ID。
    ///
    /// 返回值对应 Java `NodeSwitchComponent#getTargetList`。
    #[must_use]
    pub fn switch_target_list(&self) -> &[String] {
        &self.switch_target_list
    }

    /// 从栈顶向下查找绑定数据。
    #[must_use]
    pub fn find_bind(&self, key: &str) -> Option<&str> {
        self.binds
            .iter()
            .rev()
            .find(|(existing, _)| existing == key)
            .map(|(_, value)| value.as_str())
    }

    /// 返回最内层循环下标。
    #[must_use]
    pub fn loop_index(&self) -> Option<usize> {
        self.loops.last().map(|(index, _)| *index)
    }

    /// 返回最内层循环对象。
    #[must_use]
    pub fn loop_object(&self) -> Option<&Value> {
        self.loops.last().and_then(|(_, object)| object.as_ref())
    }

    /// 按深度返回循环对象，0 表示最内层。
    ///
    /// 参数 `depth` 对应 Java `Node#getPreNLoopObject` 的层级；不存在该层循环时
    /// 返回 `None`。对应 Java: `Node#getCurrLoopObject/getPreNLoopObject`。
    #[must_use]
    pub fn loop_object_at(&self, depth: usize) -> Option<&Value> {
        self.loops
            .len()
            .checked_sub(depth + 1)
            .and_then(|index| self.loops.get(index))
            .and_then(|(_, object)| object.as_ref())
    }

    /// 按深度返回循环下标，0 表示最内层。
    #[must_use]
    pub fn loop_index_at(&self, depth: usize) -> Option<usize> {
        self.loops
            .len()
            .checked_sub(depth + 1)
            .and_then(|index| self.loops.get(index))
            .map(|(index, _)| *index)
    }

    /// 返回携带 Chain 运行标识的新执行帧。
    ///
    /// 参数 `runtime_id` 对应 Java `Chain#runtimeIdTL` 中的纳秒级标识。
    #[must_use]
    pub fn with_runtime_id(&self, runtime_id: u64) -> Self {
        let mut frame = self.clone();
        frame.runtime_id = Some(runtime_id);
        frame
    }

    /// 返回当前 Chain 的任务隔离运行标识。
    #[must_use]
    pub fn runtime_id(&self) -> Option<u64> {
        self.runtime_id
    }

    /// 保存当前任务的 SWITCH 结果。
    pub(crate) fn set_switch_result(&mut self, key: String, value: Value) {
        self.switch_results.insert(key, value);
    }

    /// 返回当前任务的 SWITCH 结果快照。
    pub(crate) fn get_switch_result(&self, key: &str) -> Option<Value> {
        self.switch_results.get(key).cloned()
    }

    /// 保存当前任务的 IF 结果。
    pub(crate) fn set_if_result(&mut self, key: String, value: bool) {
        self.if_results.insert(key, value);
    }

    /// 返回当前任务的 IF 结果。
    pub(crate) fn get_if_result(&self, key: &str) -> Option<bool> {
        self.if_results.get(key).copied()
    }

    /// 保存当前任务的 AND/OR 结果。
    pub(crate) fn set_and_or_result(&self, key: String, value: bool) {
        self.and_or_results
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, value);
    }

    /// 返回当前任务的 AND/OR 结果。
    pub(crate) fn get_and_or_result(&self, key: &str) -> Option<bool> {
        self.and_or_results
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(key)
            .copied()
    }

    /// 保存当前任务的 NOT 结果。
    pub(crate) fn set_not_result(&self, key: String, value: bool) {
        self.not_results
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, value);
    }

    /// 返回当前任务的 NOT 结果。
    pub(crate) fn get_not_result(&self, key: &str) -> Option<bool> {
        self.not_results
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(key)
            .copied()
    }

    /// 保存当前任务的 FOR 次数。
    pub(crate) fn set_for_result(&mut self, key: String, value: usize) {
        self.for_results.insert(key, value);
    }

    /// 返回当前任务的 FOR 次数。
    pub(crate) fn get_for_result(&self, key: &str) -> Option<usize> {
        self.for_results.get(key).copied()
    }

    /// 保存当前任务的 WHILE 结果。
    pub(crate) fn set_while_result(&mut self, key: String, value: bool) {
        self.while_results.insert(key, value);
    }

    /// 返回当前任务的 WHILE 结果。
    pub(crate) fn get_while_result(&self, key: &str) -> Option<bool> {
        self.while_results.get(key).copied()
    }

    /// 保存当前任务的 BREAK 结果。
    pub(crate) fn set_break_result(&mut self, key: String, value: bool) {
        self.break_results.insert(key, value);
    }

    /// 返回当前任务的 BREAK 结果。
    pub(crate) fn get_break_result(&self, key: &str) -> Option<bool> {
        self.break_results.get(key).copied()
    }

    /// 保存当前任务的迭代队列，并取得队列所有权。
    pub(crate) fn set_iterator_result(
        &mut self,
        key: String,
        values: impl IntoIterator<Item = Value>,
    ) {
        self.iterator_results
            .insert(key, values.into_iter().collect());
    }

    /// 返回当前任务的迭代队列快照。
    pub(crate) fn get_iterator_result(&self, key: &str) -> Option<VecDeque<Value>> {
        self.iterator_results.get(key).cloned()
    }

    /// 将 Condition 压入当前任务调用栈。
    pub(crate) fn push_condition(&self, condition: Arc<dyn Condition>) {
        self.condition_stack
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(condition);
    }

    /// 弹出当前任务调用栈顶的 Condition。
    pub(crate) fn pop_condition(&self) -> Option<Arc<dyn Condition>> {
        self.condition_stack
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop()
    }

    /// 返回当前任务调用栈顶的 Condition。
    pub(crate) fn current_condition(&self) -> Option<Arc<dyn Condition>> {
        self.condition_stack
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .last()
            .cloned()
    }

    /// 返回当前任务 Condition 调用栈的浅引用快照。
    pub(crate) fn condition_stack(&self) -> Vec<Arc<dyn Condition>> {
        self.condition_stack
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// 返回当前节点预先计算的访问结果。
    pub(crate) fn get_node_access_result(&self, node_id: &str) -> bool {
        self.node_access_results
            .get(node_id)
            .copied()
            .unwrap_or(false)
    }

    /// 保存当前节点预先计算的访问结果。
    pub(crate) fn set_node_access_result(&mut self, node_id: String, result: bool) {
        self.node_access_results.insert(node_id, result);
    }

    /// 删除当前节点预先计算的访问结果。
    pub(crate) fn remove_node_access_result(&mut self, node_id: &str) {
        self.node_access_results.remove(node_id);
    }

    /// 返回当前节点预先计算的 continue-on-error 结果。
    pub(crate) fn get_node_continue_on_error_result(&self, node_id: &str) -> bool {
        self.node_continue_on_error_results
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(node_id)
            .copied()
            .unwrap_or(false)
    }

    /// 保存当前节点预先计算的 continue-on-error 结果。
    pub(crate) fn set_node_continue_on_error_result(&self, node_id: String, result: bool) {
        self.node_continue_on_error_results
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(node_id, result);
    }

    /// 删除当前节点预先计算的 continue-on-error 结果。
    pub(crate) fn remove_node_continue_on_error_result(&self, node_id: &str) {
        self.node_continue_on_error_results
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(node_id);
    }

    /// 返回当前节点的步骤自定义数据快照。
    pub(crate) fn get_node_step_data(&self, node_id: &str) -> Option<Value> {
        self.node_step_data
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(node_id)
            .cloned()
    }

    /// 保存当前节点的步骤自定义数据。
    pub(crate) fn set_node_step_data(&self, node_id: String, step_data: Value) {
        self.node_step_data
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(node_id, step_data);
    }

    /// 删除当前节点的步骤自定义数据。
    pub(crate) fn remove_node_step_data(&self, node_id: &str) {
        self.node_step_data
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(node_id);
    }

    /// 保存节点单次执行产生的结果。
    ///
    /// 参数 `node_id` 与 `item_result` 对应 Java 组件的 metaValueKey 和写入 Slot
    /// 的结果；锁中毒时保留已经完成的内部值并继续工作。
    pub(crate) fn set_node_item_result(&self, node_id: String, item_result: Value) {
        self.node_item_results
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(node_id, item_result);
    }

    /// 返回节点最近一次成功 `process` 的结果快照。
    pub(crate) fn get_node_item_result(&self, node_id: &str) -> Option<Value> {
        self.node_item_results
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(node_id)
            .cloned()
    }

    /// 删除节点执行结果。
    pub(crate) fn remove_node_item_result(&self, node_id: &str) {
        self.node_item_results
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(node_id);
    }

    /// 按 Java LoopCondition 身份写入循环下标。
    pub(crate) fn set_loop_index_for(&mut self, condition_key: usize, index: usize) {
        if let Some(position) = self
            .loop_condition_keys
            .iter()
            .position(|current| *current == Some(condition_key))
        {
            self.loops[position].0 = index;
            return;
        }
        self.loops.push((index, None));
        self.loop_condition_keys.push(Some(condition_key));
    }

    /// 按 Java LoopCondition 身份写入循环对象。
    pub(crate) fn set_loop_object_for(&mut self, condition_key: usize, object: Value) {
        if let Some(position) = self
            .loop_condition_keys
            .iter()
            .position(|current| *current == Some(condition_key))
        {
            self.loops[position].1 = Some(object);
            return;
        }
        self.loops.push((0, Some(object)));
        self.loop_condition_keys.push(Some(condition_key));
    }

    /// 弹出当前循环下标、对象及其 Condition 身份。
    pub(crate) fn pop_loop(&mut self) {
        self.loops.pop();
        self.loop_condition_keys.pop();
    }
}
