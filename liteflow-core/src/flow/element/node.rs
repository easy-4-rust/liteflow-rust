//! 对应 flow.element.Node：包装组件实例的可执行节点。
//! execute_once() 对齐 Java Node.execute → processFlow 单次执行语义：
//! isAccess → beforeProcess → process → afterProcess，
//! 异常时 onError → isContinueOnError 决定是否吞掉，全部记入 CmpStep。
//! Executable::execute 则对齐 Java Node.execute(slotIndex) 的完整入口：
//! 经 NodeExecutorHelper 取得节点执行器（对应 NodeExecutor.execute(instance)），
//! 由执行器的重试主干循环调用 execute_once。

use crate::core::node_component::NodeComponent;
use crate::el::NodeRef;
use crate::enums::{CmpStepTypeEnum, ExecuteableTypeEnum, NodeTypeEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::element::rollbackable::Rollbackable;
use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::{CmpContext, Ctx, DataBus, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::NodeHooks;

#[derive(Clone)]
pub struct Node {
    node_ref: NodeRef,
    instance: Arc<dyn NodeComponent>,
    /// 实例编号（NodeInstanceIdManageSpi；同节点多次出现时编号）
    node_instance_id: Option<String>,
    hooks: NodeHooks,
    name: String,
    clazz: Option<String>,
    node_type: Option<NodeTypeEnum>,
    script: Option<String>,
    language: Option<String>,
    curr_chain_id: Option<String>,
    is_compiled: bool,
}

impl Node {
    /// 使用节点引用和真实组件实例创建可执行节点。
    ///
    /// 参数 `node_ref` 与 `instance` 分别承载 Java Node 的规则元数据和组件实例。
    /// 对应 Java: `Node#Node(NodeComponent)`。
    pub fn new(node_ref: NodeRef, instance: Arc<dyn NodeComponent>) -> Self {
        let name = instance.name().to_string();
        let node_type = instance.node_type();
        Self {
            node_ref,
            instance,
            node_instance_id: None,
            hooks: NodeHooks::default(),
            name,
            clazz: None,
            node_type,
            script: None,
            language: None,
            curr_chain_id: None,
            // Rust 脚本组件在 FlowBus 构建节点前完成编译，因此普通构造路径已编译。
            is_compiled: true,
        }
    }

    /// 设置节点实例编号并返回节点。
    ///
    /// 参数 `instance_id` 对应 Java `nodeInstanceId`。这是构建期便利方法。
    pub fn with_instance_id(mut self, instance_id: impl Into<String>) -> Self {
        self.node_instance_id = Some(instance_id.into());
        self
    }

    /// 注入执行期切面与监控钩子并返回节点。
    pub fn with_hooks(mut self, hooks: NodeHooks) -> Self {
        self.hooks = hooks;
        self
    }

    /// 返回节点 ID。对应 Java: `Node#getId`。
    #[must_use]
    pub fn get_id(&self) -> &str {
        &self.node_ref.id
    }

    /// 设置节点 ID。
    ///
    /// 参数 `id` 对应 Java 同名参数。对应 Java: `Node#setId`。
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.node_ref.id = id.into();
    }

    /// 返回节点实例编号；尚未分配时返回 `None`。
    ///
    /// 对应 Java: `Node#getNodeInstanceId`。
    #[must_use]
    pub fn get_node_instance_id(&self) -> Option<&str> {
        self.node_instance_id.as_deref()
    }

    /// 返回节点实例编号；是 Java 命名入口的 Rust 兼容别名。
    #[must_use]
    pub fn node_instance_id(&self) -> Option<&str> {
        self.get_node_instance_id()
    }

    /// 设置节点实例编号。
    ///
    /// 参数 `node_instance_id` 对应 Java 同名参数。
    /// 对应 Java: `Node#setNodeInstanceId`。
    pub fn set_node_instance_id(&mut self, instance_id: impl Into<String>) {
        self.node_instance_id = Some(instance_id.into());
    }

    /// 返回节点标签；未配置时返回 `None`。对应 Java: `Node#getTag`。
    #[must_use]
    pub fn get_tag(&self) -> Option<&str> {
        self.node_ref.tag.as_deref()
    }

    /// 设置节点标签。
    ///
    /// 参数 `tag` 对应 Java 同名参数。对应 Java: `Node#setTag`。
    pub fn set_tag(&mut self, tag: impl Into<String>) {
        self.node_ref.tag = Some(tag.into());
    }

    /// 返回节点名称。对应 Java: `Node#getName`。
    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// 设置节点名称。
    ///
    /// 参数 `name` 对应 Java 同名参数。对应 Java: `Node#setName`。
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// 返回节点类型；组件未声明类型时返回 `None`。
    ///
    /// 对应 Java: `Node#getType`。
    #[must_use]
    pub fn get_type(&self) -> Option<NodeTypeEnum> {
        self.node_type
    }

    /// 设置节点类型。
    ///
    /// 参数 `node_type` 对应 Java `type` 参数。对应 Java: `Node#setType`。
    pub fn set_type(&mut self, node_type: NodeTypeEnum) {
        self.node_type = Some(node_type);
    }

    /// 返回真实组件实例。
    ///
    /// Rust 在规则构建阶段完成脚本组件编译，因此这里无需 Java 的双重检查锁。
    /// 对应 Java: `Node#getInstance`。
    #[must_use]
    pub fn get_instance(&self) -> &Arc<dyn NodeComponent> {
        &self.instance
    }

    /// 替换真实组件实例。
    ///
    /// 参数 `instance` 对应 Java 同名参数。对应 Java: `Node#setInstance`。
    pub fn set_instance(&mut self, instance: Arc<dyn NodeComponent>) {
        self.instance = instance;
    }

    /// 返回脚本内容；非脚本节点返回 `None`。对应 Java: `Node#getScript`。
    #[must_use]
    pub fn get_script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    /// 设置脚本内容并标记为尚未编译。
    ///
    /// 参数 `script` 对应 Java 同名参数。对应 Java: `Node#setScript`。
    pub fn set_script(&mut self, script: impl Into<String>) {
        self.script = Some(script.into());
        self.is_compiled = false;
    }

    /// 返回 Java 组件类名的 Rust 诊断映射。
    ///
    /// Rust 组件没有 JVM Class；未显式登记时返回 `None`。
    /// 对应 Java: `Node#getClazz`。
    #[must_use]
    pub fn get_clazz(&self) -> Option<&str> {
        self.clazz.as_deref()
    }

    /// 设置组件诊断类名。
    ///
    /// 参数 `clazz` 对应 Java 同名参数。对应 Java: `Node#setClazz`。
    pub fn set_clazz(&mut self, clazz: impl Into<String>) {
        self.clazz = Some(clazz.into());
    }

    /// 返回组件数据。对应 Java: `Node#getCmpData`。
    #[must_use]
    pub fn get_cmp_data(&self) -> Option<&str> {
        self.node_ref.data.as_deref()
    }

    /// 设置组件数据。
    ///
    /// 参数 `cmp_data` 对应 Java 同名参数。对应 Java: `Node#setCmpData`。
    pub fn set_cmp_data(&mut self, cmp_data: impl Into<String>) {
        self.node_ref.data = Some(cmp_data.into());
    }

    /// 设置当前 Chain ID。
    ///
    /// 参数 `current_chain_id` 对应 Java 同名参数。
    /// 对应 Java: `Node#setCurrChainId`。
    pub fn set_curr_chain_id(&mut self, current_chain_id: impl Into<String>) {
        self.curr_chain_id = Some(current_chain_id.into());
    }

    /// 返回当前 Chain ID。对应 Java: `Node#getCurrChainId`。
    #[must_use]
    pub fn get_curr_chain_id(&self) -> Option<&str> {
        self.curr_chain_id.as_deref()
    }

    /// 返回脚本语言；非脚本节点返回 `None`。对应 Java: `Node#getLanguage`。
    #[must_use]
    pub fn get_language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// 设置脚本语言。
    ///
    /// 参数 `language` 对应 Java 同名参数。对应 Java: `Node#setLanguage`。
    pub fn set_language(&mut self, language: impl Into<String>) {
        self.language = Some(language.into());
    }

    /// 返回脚本节点是否已经编译。对应 Java: `Node#isCompiled`。
    #[must_use]
    pub fn is_compiled(&self) -> bool {
        self.is_compiled
    }

    /// 设置脚本编译状态。
    ///
    /// 参数 `compiled` 对应 Java 同名参数。对应 Java: `Node#setCompiled`。
    pub fn set_compiled(&mut self, compiled: bool) {
        self.is_compiled = compiled;
    }

    /// 写入节点绑定数据。
    ///
    /// 同名键覆盖原值。参数 `key`、`value` 对应 Java 同名参数。
    /// 对应 Java: `Node#putBindData`。
    pub fn put_bind_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        if let Some((_, current)) = self
            .node_ref
            .bind
            .iter_mut()
            .find(|(current_key, _)| current_key == &key)
        {
            *current = value;
        } else {
            self.node_ref.bind.push((key, value));
        }
    }

    /// 判断指定键是否存在绑定数据。
    ///
    /// 参数 `key` 为绑定键。对应 Java: `Node#hasBindData`。
    #[must_use]
    pub fn has_bind_data(&self, key: &str) -> bool {
        self.node_ref
            .bind
            .iter()
            .any(|(current_key, _)| current_key == key)
    }

    /// 返回指定键的绑定数据。
    ///
    /// 参数 `key` 为绑定键；不存在时返回 `None`。对应 Java: `Node#getBindData`。
    #[must_use]
    pub fn get_bind_data(&self, key: &str) -> Option<&str> {
        self.node_ref
            .bind
            .iter()
            .find(|(current_key, _)| current_key == key)
            .map(|(_, value)| value.as_str())
    }

    /// 删除指定键的绑定数据。
    ///
    /// 参数 `key` 为绑定键。对应 Java: `Node#removeBindData`。
    pub fn remove_bind_data(&mut self, key: &str) {
        self.node_ref
            .bind
            .retain(|(current_key, _)| current_key != key);
    }

    /// 返回可执行元素类型。对应 Java: `Node#getExecuteType`。
    #[must_use]
    pub fn get_execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Node
    }

    /// 执行节点主逻辑。
    ///
    /// Java 通过 `slot_index` 从 DataBus 反查执行上下文；Rust 在异步边界显式传递
    /// `ctx` 与 `frame`，避免索引释放和任务迁移造成悬空状态。
    /// 对应 Java: `Node#execute`。
    pub async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        <Self as Executable>::execute(self, ctx, frame).await
    }

    /// 执行节点回滚并记录回滚步骤。
    ///
    /// 参数 `ctx`、`frame` 是 Java `slot_index` 在 Rust 中的安全执行上下文映射。
    /// 对应 Java: `Node#rollback`。
    pub async fn rollback(&self, ctx: &Ctx, frame: &Frame) -> LFResult<()> {
        <Self as Rollbackable>::rollback(self, ctx, frame).await
    }

    /// 判断节点在当前执行上下文中是否可进入。
    ///
    /// Java 的 `slot_index` 映射为显式 `ctx` 与 `frame`。
    /// 对应 Java: `Node#isAccess`。
    pub async fn is_access(&self, ctx: &Ctx, frame: &Frame) -> bool {
        <Self as Executable>::is_access(self, ctx, frame).await
    }

    /// 返回最内层循环下标。
    ///
    /// 参数 `frame` 承载 Java `loopIndexTL` 的任务隔离栈。
    /// 对应 Java: `Node#getLoopIndex`。
    #[must_use]
    pub fn get_loop_index(&self, frame: &Frame) -> Option<usize> {
        frame.loop_index()
    }

    /// 返回上一层循环下标。对应 Java: `Node#getPreLoopIndex`。
    #[must_use]
    pub fn get_pre_loop_index(&self, frame: &Frame) -> Option<usize> {
        self.get_pre_n_loop_index(frame, 1)
    }

    /// 返回向外第 `n` 层循环下标。
    ///
    /// `n=0` 表示当前层。对应 Java: `Node#getPreNLoopIndex`。
    #[must_use]
    pub fn get_pre_n_loop_index(&self, frame: &Frame, n: usize) -> Option<usize> {
        frame.loop_index_at(n)
    }

    /// 返回最内层循环对象。
    ///
    /// 参数 `frame` 承载 Java `loopObjectTL` 的任务隔离栈。
    /// 对应 Java: `Node#getCurrLoopObject`。
    #[must_use]
    pub fn get_curr_loop_object<'a>(&self, frame: &'a Frame) -> Option<&'a Value> {
        frame.loop_object()
    }

    /// 返回上一层循环对象。对应 Java: `Node#getPreLoopObject`。
    #[must_use]
    pub fn get_pre_loop_object<'a>(&self, frame: &'a Frame) -> Option<&'a Value> {
        self.get_pre_n_loop_object(frame, 1)
    }

    /// 返回向外第 `n` 层循环对象。
    ///
    /// `n=0` 表示当前层。对应 Java: `Node#getPreNLoopObject`。
    #[must_use]
    pub fn get_pre_n_loop_object<'a>(&self, frame: &'a Frame, n: usize) -> Option<&'a Value> {
        frame
            .loops
            .len()
            .checked_sub(n + 1)
            .and_then(|index| frame.loops.get(index))
            .and_then(|(_, object)| object.as_ref())
    }

    /// 返回提前计算的 isAccess 结果。
    ///
    /// 参数 `frame` 映射 Java `TransmittableThreadLocal`。
    /// 对应 Java: `Node#getAccessResult`。
    #[must_use]
    pub fn get_access_result(&self, frame: &Frame) -> bool {
        frame.get_node_access_result(self.get_id())
    }

    /// 保存提前计算的 isAccess 结果。
    ///
    /// 参数 `access_result` 对应 Java 同名参数。对应 Java: `Node#setAccessResult`。
    pub fn set_access_result(&self, frame: &mut Frame, access_result: bool) {
        frame.set_node_access_result(self.get_id().to_string(), access_result);
    }

    /// 删除提前计算的 isAccess 结果。对应 Java: `Node#removeAccessResult`。
    pub fn remove_access_result(&self, frame: &mut Frame) {
        frame.remove_node_access_result(self.get_id());
    }

    /// 返回提前计算的 continue-on-error 结果。
    ///
    /// 对应 Java: `Node#getIsContinueOnErrorResult`。
    #[must_use]
    pub fn get_is_continue_on_error_result(&self, frame: &Frame) -> bool {
        frame.get_node_continue_on_error_result(self.get_id())
    }

    /// 保存提前计算的 continue-on-error 结果。
    ///
    /// 参数 `is_continue_on_error_result` 对应 Java 状态值。
    /// 对应 Java: `Node#setIsContinueOnErrorResult`。
    pub fn set_is_continue_on_error_result(
        &self,
        frame: &Frame,
        is_continue_on_error_result: bool,
    ) {
        frame.set_node_continue_on_error_result(
            self.get_id().to_string(),
            is_continue_on_error_result,
        );
    }

    /// 删除提前计算的 continue-on-error 结果。
    ///
    /// 对应 Java: `Node#removeIsContinueOnErrorResult`。
    pub fn remove_is_continue_on_error_result(&self, frame: &Frame) {
        frame.remove_node_continue_on_error_result(self.get_id());
    }

    /// 设置当前循环下标。
    ///
    /// `condition_key` 对应 Java `LoopCondition#hashCode`，显式 Frame 已限定当前
    /// 条件作用域，因此仅用于保持参数语义。对应 Java: `Node#setLoopIndex`。
    pub fn set_loop_index(&self, frame: &mut Frame, condition_key: usize, index: usize) {
        frame.set_loop_index_for(condition_key, index);
    }

    /// 删除当前循环下标及其关联对象。对应 Java: `Node#removeLoopIndex`。
    pub fn remove_loop_index(&self, frame: &mut Frame) {
        frame.pop_loop();
    }

    /// 设置当前循环对象。
    ///
    /// 参数 `condition_key` 对应 Java 条件身份，`object` 对应 Java `obj`。
    /// 对应 Java: `Node#setCurrLoopObject`。
    pub fn set_curr_loop_object(&self, frame: &mut Frame, condition_key: usize, object: Value) {
        frame.set_loop_object_for(condition_key, object);
    }

    /// 删除当前循环对象及其关联下标。对应 Java: `Node#removeCurrLoopObject`。
    pub fn remove_curr_loop_object(&self, frame: &mut Frame) {
        frame.pop_loop();
    }

    /// 返回当前 Slot 在 DataBus 中的索引。
    ///
    /// 参数 `ctx` 映射 Java Node 的 slotIndex ThreadLocal。
    /// 对应 Java: `Node#getSlotIndex`。
    #[must_use]
    pub fn get_slot_index(&self, ctx: &Ctx) -> Option<usize> {
        DataBus::get_slot_index(&ctx.inner)
    }

    /// 根据 Slot 索引建立安全执行上下文。
    ///
    /// Java 写入 ThreadLocal；Rust 返回持有 `Arc<Slot>` 的 `Ctx`，索引无效时
    /// 返回 `None`。对应 Java: `Node#setSlotIndex`。
    #[must_use]
    pub fn set_slot_index(&self, slot_index: usize) -> Option<Ctx> {
        DataBus::get_slot(slot_index).map(Ctx::new)
    }

    /// 删除调用方持有的 Slot 执行上下文。
    ///
    /// 对应 Java: `Node#removeSlotIndex`。
    pub fn remove_slot_index(&self, ctx: &mut Option<Ctx>) {
        *ctx = None;
    }

    /// 返回当前执行是否已经请求结束整个串行流程。
    ///
    /// 对应 Java: `Node#getIsEnd`。
    #[must_use]
    pub fn get_is_end(&self, ctx: &Ctx) -> bool {
        ctx.inner.ended.load(Ordering::Acquire)
    }

    /// 设置是否结束整个串行流程。
    ///
    /// 参数 `is_end` 对应 Java 同名参数。对应 Java: `Node#setIsEnd`。
    pub fn set_is_end(&self, ctx: &Ctx, is_end: bool) {
        ctx.inner.ended.store(is_end, Ordering::Release);
    }

    /// 清除当前流程结束标记。对应 Java: `Node#removeIsEnd`。
    pub fn remove_is_end(&self, ctx: &Ctx) {
        ctx.inner.ended.store(false, Ordering::Release);
    }

    /// 返回当前节点的步骤自定义数据。
    ///
    /// 对应 Java: `Node#getStepData`。
    #[must_use]
    pub fn get_step_data(&self, frame: &Frame) -> Option<Value> {
        frame.get_node_step_data(self.get_id())
    }

    /// 设置当前节点的步骤自定义数据。
    ///
    /// 参数 `step_data` 对应 Java 同名参数。对应 Java: `Node#setStepData`。
    pub fn set_step_data(&self, frame: &Frame, step_data: Value) {
        frame.set_node_step_data(self.get_id().to_string(), step_data);
    }

    /// 删除当前节点的步骤自定义数据。对应 Java: `Node#removeStepData`。
    pub fn remove_step_data(&self, frame: &Frame) {
        frame.remove_node_step_data(self.get_id());
    }

    /// 返回当前任务中本节点最近一次成功 `process` 的结果。
    ///
    /// Java 的 Boolean/Switch/For/Iterator 组件先把结果写入 Slot，再由本方法
    /// 委托组件读取；Rust 将同一职责映射到显式 `Frame` 结果表。该方法只读取
    /// 单次执行缓存，不会再次调用有副作用的组件。
    ///
    /// 参数 `frame` 对应 Java `slot_index` 定位到的任务状态；未执行或未产生
    /// 结果时返回 `None`。对应 Java: `Node#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, frame: &Frame) -> Option<Value> {
        frame.get_node_item_result(self.get_id())
    }

    /// 删除当前任务缓存的节点执行结果。
    ///
    /// Java 在 Slot 生命周期结束时整体释放结果表；Rust 同时提供节点级清理，
    /// 供长生命周期 Frame 或测试显式回收。
    pub fn remove_item_result_meta_value(&self, frame: &Frame) {
        frame.remove_node_item_result(self.get_id());
    }

    /// 克隆节点定义和绑定数据，执行期上下文不会被复制。
    ///
    /// Rust 的 `Ctx/Frame` 本来就按执行分配，因此天然满足 Java clone 重建所有
    /// ThreadLocal 的隔离语义。对应 Java: `Node#clone`。
    #[must_use]
    pub fn clone(&self) -> Self {
        <Self as Clone>::clone(self)
    }

    /// 复制节点；语义与 Java 构建阶段的 `Node#clone` 一致。
    ///
    /// 对应 Java: `Node#copy`。
    #[must_use]
    pub fn copy(&self) -> Self {
        <Self as Clone>::clone(self)
    }

    /// 返回节点规则引用。
    #[must_use]
    pub fn node_ref(&self) -> &NodeRef {
        &self.node_ref
    }

    /// 返回真实组件实例；是 `get_instance` 的 Rust 兼容别名。
    #[must_use]
    pub fn instance(&self) -> &Arc<dyn NodeComponent> {
        self.get_instance()
    }

    /// getDisplayName()（优先别名）
    pub fn display_name(&self) -> &str {
        self.node_ref.display()
    }

    /// 单次执行逻辑（对应 Java NodeComponent.execute() 被 NodeExecutor 重试循环
    /// 反复调用的那一次执行）：isAccess → beforeProcess → process → afterProcess，
    /// 异常时 onError → isContinueOnError 决定是否吞掉，全部记入 CmpStep。
    /// 重试语义由 flow.executor.NodeExecutor 承担，本方法不含重试。
    pub async fn execute_once(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        if ctx.is_ended() {
            return Err(LiteflowError::ChainEnd);
        }
        let cctx = CmpContext {
            inner: ctx.inner.clone(),
            node: self.node_ref.clone(),
            frame: frame.clone(),
        };

        // isAccess
        if !self.instance.is_access(&cctx) {
            return Ok(Value::Null);
        }

        let mut step = CmpStep::new(
            self.display_name().to_string(),
            self.instance.name(),
            CmpStepTypeEnum::Single,
        );
        step.node_instance_id = self.node_instance_id.clone();
        step.tag = self.node_ref.tag.clone();
        step.set_instance(self.instance.clone());
        step.set_ref_node(self.clone());

        // Java 在 NodeComponent.execute() 开始时就把 instance/refNode 写入 CmpStep。
        // Rust 端显式登记内部回滚目标；重试会重复登记，但真正回滚时按
        // NodeInstanceId 去重，对齐 NodeComponent#doRollback。
        if self.instance.is_rollback() {
            let node_instance_id = self
                .node_instance_id
                .clone()
                .unwrap_or_else(|| self.node_ref.display().to_string());
            ctx.register_rollback(node_instance_id, self.instance.clone(), cctx.clone());
        }

        // 全局切面 beforeProcess（对应 aop.ICmpAroundAspect）
        for aspect in &self.hooks.aspects {
            aspect.before_process(&cctx).await;
        }

        // 对齐 Java NodeComponent#execute：
        // beforeProcess → process → onSuccess；任一步骤失败都进入 onError；
        // afterProcess 始终在 finally 语义中执行。
        let result = match self.instance.before_process(&cctx).await {
            Ok(()) => match self.instance.process(&cctx).await {
                Ok(value) => {
                    // Java 四类有返回值组件在 process 内先写 Slot，再执行
                    // onSuccess；因此即使 onSuccess 失败，已经产生的结果仍可读取。
                    frame.set_node_item_result(self.get_id().to_string(), value.clone());
                    self.instance.on_success(&cctx).await.map(|_| value)
                }
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };

        match &result {
            Ok(_) => {
                for aspect in &self.hooks.aspects {
                    aspect.on_success(&cctx).await;
                }
            }
            Err(error) => {
                self.instance.on_error(&cctx, error).await;
                for aspect in &self.hooks.aspects {
                    aspect.on_error(&cctx, error).await;
                }
                if !matches!(error, LiteflowError::ChainEnd) {
                    ctx.set_exception(&error.to_string());
                }
            }
        }

        self.instance.after_process(&cctx).await;
        for aspect in &self.hooks.aspects {
            aspect.after_process(&cctx).await;
        }

        // Java Node 在成功和异常两条路径都会再次调用组件 isEnd；脚本执行器可在
        // process 之外动态决定结束流程。命中后写回共享 Slot，确保后续分支立即
        // 观察到结束状态，并让 ChainEnd 优先于普通错误/continue-on-error。
        if self.instance.is_end(&cctx) {
            ctx.inner.ended.store(true, Ordering::Release);
        }

        match result {
            Ok(v) => {
                step.set_step_data(cctx.get_step_data().unwrap_or(serde_json::Value::Null));
                step.finish(true, None);
                if let Some(m) = &self.hooks.monitor {
                    m.record(
                        self.display_name(),
                        step.time_spent.unwrap_or_default(),
                        true,
                    );
                }
                ctx.record_step(step);
                // setIsEnd(true) 语义
                if ctx.is_ended() {
                    return Err(LiteflowError::ChainEnd);
                }
                Ok(v)
            }
            Err(LiteflowError::ChainEnd) => {
                step.set_step_data(cctx.get_step_data().unwrap_or(serde_json::Value::Null));
                step.finish(false, Some(LiteflowError::ChainEnd.to_string()));
                if let Some(m) = &self.hooks.monitor {
                    m.record(
                        self.display_name(),
                        step.time_spent.unwrap_or_default(),
                        false,
                    );
                }
                ctx.record_step(step);
                Err(LiteflowError::ChainEnd)
            }
            Err(e) => {
                let error_kind = format!("{e:?}")
                    .split([' ', '(', '{'])
                    .next()
                    .unwrap_or_default()
                    .to_string();
                step.set_step_data(cctx.get_step_data().unwrap_or(serde_json::Value::Null));
                step.finish(false, Some(e.to_string()));
                if let Some(m) = &self.hooks.monitor {
                    m.record(
                        self.display_name(),
                        step.time_spent.unwrap_or_default(),
                        false,
                    );
                }
                ctx.record_step(step);
                if ctx.is_ended() {
                    return Err(LiteflowError::ChainEnd);
                }
                if cctx.frame.get_node_continue_on_error_result(self.get_id())
                    || self.instance.is_continue_on_error_with_context(&cctx)
                {
                    return Ok(Value::Null);
                }
                Err(LiteflowError::NodeExec {
                    node: self.display_name().to_string(),
                    msg: e.to_string(),
                    kind: error_kind,
                })
            }
        }
    }
}

#[async_trait]
impl Executable for Node {
    /// 对应 Java Node.execute(slotIndex)：经 NodeExecutorHelper.buildNodeExecutor
    /// 取得节点执行器（组件未指定时为缓存的 DefaultNodeExecutor），
    /// 委托执行器的重试主干执行（NodeExecutor.execute → 循环调用 execute_once）。
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let executor = crate::flow::executor::NodeExecutorHelper::load_instance()
            .build_node_executor(self.instance.node_executor());
        executor.execute(self, ctx, frame).await
    }

    fn execute_type(&self) -> crate::enums::ExecuteableTypeEnum {
        crate::enums::ExecuteableTypeEnum::Node
    }

    fn id(&self) -> &str {
        self.node_ref.display()
    }

    fn tag(&self) -> Option<&str> {
        self.node_ref.tag.as_deref()
    }

    /// isAccess(slotIndex)（2.16：AND/OR 求值前的过滤依据）
    async fn is_access(&self, ctx: &Ctx, frame: &Frame) -> bool {
        let cctx = CmpContext {
            inner: ctx.inner.clone(),
            node: self.node_ref.clone(),
            frame: frame.clone(),
        };
        self.instance.is_access(&cctx)
    }
}

#[async_trait]
impl Rollbackable for Node {
    /// 调用组件补偿逻辑并记录 rollback step。
    ///
    /// 与 Java `Node#rollback` 一致，组件回滚错误只记录为失败步骤，不覆盖触发
    /// 补偿的原始流程错误。
    async fn rollback(&self, ctx: &Ctx, frame: &Frame) -> LFResult<()> {
        let component_context = CmpContext {
            inner: ctx.inner.clone(),
            node: self.node_ref.clone(),
            frame: frame.clone(),
        };
        let mut step = CmpStep::new(
            self.display_name().to_string(),
            self.instance.name(),
            CmpStepTypeEnum::Single,
        );
        step.node_instance_id = self.node_instance_id.clone();
        step.tag = self.node_ref.tag.clone();
        step.set_instance(self.instance.clone());
        step.set_ref_node(self.clone());

        match self.instance.rollback(&component_context).await {
            Ok(()) => step.finish_rollback(true, None),
            Err(error) => step.finish_rollback(false, Some(error.to_string())),
        }
        if let Ok(mut rollback_steps) = ctx.inner.rollback_steps.lock() {
            rollback_steps.push(step);
        }
        Ok(())
    }
}
