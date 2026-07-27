//! 对应 Java `core.NodeComponent` 基类。
//!
//! 四个有返回值的 Java 子类已经拆到各自文件，并在适配到本 trait 时用
//! `serde_json::Value` 传递类型结果：
//! - 普通组件 → `Value::Null`
//! - 布尔组件（IF/WHILE/BREAK/AND/OR/NOT）→ `Value::Bool`
//! - SWITCH 组件 → `Value::String`（目标 id，可带 "id:tag"）
//! - FOR 组件 → 数字
//! - ITERATOR 组件 → `Value::Array`

use crate::el::NodeRef;
use crate::enums::NodeTypeEnum;
use crate::exception::LiteflowError;
use crate::flow::executor::NodeExecutor;
use crate::slot::{CmpContext, Slot};
use crate::util::LiteflowContextRegexMatcher;
use async_trait::async_trait;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[async_trait]
pub trait NodeComponent: Send + Sync + 'static {
    /// process() / processIf() / processSwitch() / processFor() / processIterator()
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>;

    /// beforeProcess()
    async fn before_process(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }
    /// onSuccess()：`process` 成功后的回调。
    ///
    /// 对应 Java: `com.yomahub.liteflow.core.NodeComponent#onSuccess`。
    /// 回调抛错时按组件执行失败处理，并继续进入 `on_error` 与 `after_process`。
    async fn on_success(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }
    /// afterProcess()
    async fn after_process(&self, ctx: &CmpContext) {
        // 默认组件没有收尾副作用；实现方可覆盖该钩子。
        let _ = ctx;
    }
    /// onError()
    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        // 默认组件不吞掉也不改写错误；错误仍由 Node 执行主干传播。
        let _ = (ctx, error);
    }
    /// isAccess()
    fn is_access(&self, _ctx: &CmpContext) -> bool {
        true
    }
    /// isContinueOnError()
    fn is_continue_on_error(&self) -> bool {
        false
    }
    /// 根据当前任务上下文判断异常后是否继续。
    ///
    /// 普通组件沿用静态 `is_continue_on_error`；脚本组件可把当前 Slot/Frame
    /// 快照交给脚本执行器。参数 `ctx` 对应 Java 组件 ThreadLocal 中的本次执行
    /// 上下文。对应 Java: `NodeComponent#isContinueOnError`。
    fn is_continue_on_error_with_context(&self, ctx: &CmpContext) -> bool {
        let _ = ctx;
        self.is_continue_on_error()
    }
    /// 是否需要失败补偿。
    ///
    /// Java 构造器通过反射判断组件是否覆盖 `rollback()`；Rust trait 无法在运行时
    /// 可靠判断默认方法是否被覆盖，因此显式返回该能力。后续 `liteflow-derive`
    /// 会在声明了 rollback 方法时自动生成此标记。
    fn is_rollback(&self) -> bool {
        false
    }
    /// Rollbackable.rollback()。
    ///
    /// 对应 Java: `com.yomahub.liteflow.core.NodeComponent#rollback`。
    async fn rollback(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }
    /// getName()
    fn name(&self) -> &str {
        ""
    }
    /// getNodeId()：返回初始化器写入的节点 id。
    fn node_id(&self) -> &str {
        ""
    }
    /// getType()：返回初始化器写入的节点类型。
    fn node_type(&self) -> Option<NodeTypeEnum> {
        None
    }
    /// 节点类型是否由注册入口或组件元数据显式声明。
    ///
    /// Rust 的 `cmp` 闭包没有 Java class 可供反射推断，FlowBus 会根据 EL 所在位置
    /// 推断其执行类型；derive、脚本和 `add_node` 注册的组件则返回 `true`，必须在
    /// 构建期接受严格的 OperatorHelper 类型校验。
    fn has_explicit_node_type(&self) -> bool {
        self.node_type().is_some()
    }
    /// getRetryCount()：最大重试次数（默认 0 = 不重试，总尝试次数 = retry_count + 1）
    fn retry_count(&self) -> usize {
        0
    }
    /// getRetryForExceptions() 语义：判断抛出的异常是否命中组件声明的可重试异常范围
    /// （Java 用 retryForExceptions 列表 + isAssignableFrom 判定，Rust 化为谓词方法）
    fn is_retry_for(&self, _e: &LiteflowError) -> bool {
        false
    }
    /// getNodeExecutorClass()：指定自定义节点执行器；None 表示使用 DefaultNodeExecutor
    /// （Java 返回 Class 由 NodeExecutorHelper 经 DI 容器实例化并缓存，
    /// Rust 端无 DI 容器，直接提供 Arc 实例）
    fn node_executor(&self) -> Option<Arc<dyn NodeExecutor>> {
        None
    }

    /// 卸载组件持有的脚本编译产物。
    ///
    /// 普通组件没有脚本缓存，返回 `false`；脚本组件卸载成功后返回 `true`。
    /// 该对象安全入口让 `FlowBus#unloadScriptNode` 能在不知道具体脚本引擎类型的
    /// 情况下执行真实清理。
    fn unload_script(&self, _node_id: &str) -> Result<bool, LiteflowError> {
        Ok(false)
    }

    /// 返回当前节点是否已经请求结束整个流程。
    ///
    /// - `ctx`: 当前组件执行上下文。
    /// - 返回：`true` 表示 Node 主干应抛出 `ChainEnd`。
    ///
    /// 对应 Java: `NodeComponent#isEnd`。
    fn is_end(&self, ctx: &CmpContext) -> bool {
        ctx.inner.ended.load(Ordering::Acquire)
    }

    /// 设置是否结束整个流程。
    ///
    /// - `ctx`: 当前组件执行上下文。
    /// - `is_end`: 是否在本节点后主动终止链路。
    ///
    /// 对应 Java: `NodeComponent#setIsEnd`。
    fn set_is_end(&self, ctx: &CmpContext, is_end: bool) {
        ctx.inner.ended.store(is_end, Ordering::Release);
    }

    /// 动态设置当前节点异常后是否继续执行。
    ///
    /// 该值写入当前任务 Frame，优先级高于组件类型的静态
    /// `is_continue_on_error` 返回值。对应 Java:
    /// `NodeComponent#setIsContinueOnError`。
    fn set_is_continue_on_error(&self, ctx: &CmpContext, is_continue_on_error: bool) {
        ctx.frame.set_node_continue_on_error_result(
            ctx.node.display().to_string(),
            is_continue_on_error,
        );
    }

    /// 返回当前 Slot 在 DataBus 中的索引。
    ///
    /// Slot 已释放时返回 `None`。对应 Java: `NodeComponent#getSlotIndex`。
    fn get_slot_index(&self, ctx: &CmpContext) -> Option<usize> {
        ctx.slot_index()
    }

    /// 返回当前组件共享的 Slot。
    ///
    /// Rust 使用 `Arc` 表达 Java DataBus 返回的共享对象。对应 Java:
    /// `NodeComponent#getSlot`。
    fn get_slot(&self, ctx: &CmpContext) -> Arc<Slot> {
        Arc::clone(&ctx.inner)
    }

    /// 返回按插入顺序登记的第一个上下文 Bean。
    ///
    /// Bean 保留真实运行时类型，调用方可通过 `Arc::downcast` 取得具体对象。
    /// 对应 Java: `NodeComponent#getFirstContextBean`。
    fn get_first_context_bean(&self, ctx: &CmpContext) -> Option<Arc<dyn Any + Send + Sync>> {
        ctx.inner
            .get_context_bean_list()
            .into_iter()
            .next()
            .map(|(_, bean)| bean)
    }

    /// 按名称返回上下文 Bean。
    ///
    /// - `ctx`: 当前组件执行上下文。
    /// - `context_name`: Java `contextName` 参数。
    ///
    /// 对应 Java: `NodeComponent#getContextBean(String)`。
    fn get_context_bean(
        &self,
        ctx: &CmpContext,
        context_name: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        ctx.inner
            .get_context_bean_list()
            .into_iter()
            .find(|(name, _)| name == context_name)
            .map(|(_, bean)| bean)
    }

    /// 返回组件初始化后的节点 ID。对应 Java: `NodeComponent#getNodeId`。
    fn get_node_id(&self) -> &str {
        self.node_id()
    }

    /// 返回组件初始化后的名称。对应 Java: `NodeComponent#getName`。
    fn get_name(&self) -> &str {
        self.name()
    }

    /// 返回组件初始化后的节点类型。对应 Java: `NodeComponent#getType`。
    fn get_type(&self) -> Option<NodeTypeEnum> {
        self.node_type()
    }

    /// 向指定节点的私有传递队列追加数据。
    ///
    /// - `node_id`: 接收节点 ID。
    /// - `delivery_data`: 按 serde JSON 边界传递的数据。
    ///
    /// 对应 Java: `NodeComponent#sendPrivateDeliveryData`。
    fn send_private_delivery_data(&self, ctx: &CmpContext, node_id: &str, delivery_data: Value) {
        ctx.inner.set_private_delivery_data(node_id, delivery_data);
    }

    /// 取出发送给当前节点的首条私有传递数据。
    ///
    /// 队列为空时返回 `None`。对应 Java:
    /// `NodeComponent#getPrivateDeliveryData`。
    fn get_private_delivery_data(&self, ctx: &CmpContext) -> Option<Value> {
        let node_id = if self.node_id().is_empty() {
            ctx.node_id()
        } else {
            self.node_id()
        };
        ctx.inner.get_private_delivery_data(node_id)
    }

    /// 返回最大重试次数。对应 Java: `NodeComponent#getRetryCount`。
    fn get_retry_count(&self) -> usize {
        self.retry_count()
    }

    /// 返回当前组件指定的节点执行器。
    ///
    /// Rust 直接返回共享执行器对象，映射 Java 返回执行器 Class 后再由 Holder
    /// 实例化的流程。对应 Java: `NodeComponent#getNodeExecutorClass`。
    fn get_node_executor_class(&self) -> Option<Arc<dyn NodeExecutor>> {
        self.node_executor()
    }

    /// 返回当前节点标签。对应 Java: `NodeComponent#getTag`。
    fn get_tag<'a>(&self, ctx: &'a CmpContext) -> Option<&'a str> {
        ctx.tag()
    }

    /// 返回当前主链的请求数据。
    ///
    /// Java 从 `Slot#getChainReqData(getChainId())` 读取；Rust 原样返回 serde
    /// JSON 值。对应 Java: `NodeComponent#getRequestData`。
    fn get_request_data(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.inner.get_chain_req_data(ctx.chain_id())
    }

    /// 返回当前主链 ID。对应 Java: `NodeComponent#getChainId`。
    fn get_chain_id<'a>(&self, ctx: &'a CmpContext) -> &'a str {
        ctx.chain_id()
    }

    /// 返回当前主链 ID 的废弃兼容入口。
    ///
    /// 对应 Java: `NodeComponent#getChainName`。
    #[deprecated(note = "请使用 get_chain_id")]
    fn get_chain_name<'a>(&self, ctx: &'a CmpContext) -> &'a str {
        self.get_chain_id(ctx)
    }

    /// 返回组件展示名称。
    ///
    /// 名称为空时仅返回 nodeId，否则返回 `nodeId(name)`。对应 Java:
    /// `NodeComponent#getDisplayName`。
    fn get_display_name(&self) -> String {
        if self.name().is_empty() {
            self.node_id().to_string()
        } else {
            format!("{}({})", self.node_id(), self.name())
        }
    }

    /// 返回当前实际执行的 Chain ID。
    ///
    /// 子链中返回子链 ID，而 `get_chain_id` 仍返回 Slot 主链 ID。对应 Java:
    /// `NodeComponent#getCurrChainId`。
    fn get_curr_chain_id<'a>(&self, ctx: &'a CmpContext) -> &'a str {
        ctx.curr_chain_id()
    }

    /// 返回当前节点规则引用。对应 Java: `NodeComponent#getRefNode`。
    fn get_ref_node<'a>(&self, ctx: &'a CmpContext) -> &'a NodeRef {
        &ctx.node
    }

    /// 返回当前节点的原始组件数据。
    ///
    /// 对应 Java `getCmpData(String.class)`；空值返回 `None`。
    fn get_cmp_data(&self, ctx: &CmpContext) -> Option<String> {
        ctx.cmp_data().map(ToOwned::to_owned)
    }

    /// 将当前节点组件数据解析为 JSON 数组。
    ///
    /// 非数组或非法 JSON 返回 `None`，对应 Java:
    /// `NodeComponent#getCmpDataList` 的 serde 映射。
    fn get_cmp_data_list(&self, ctx: &CmpContext) -> Option<Vec<Value>> {
        serde_json::from_str(ctx.cmp_data()?).ok()
    }

    /// 返回指定 key 的绑定数据。
    ///
    /// 先查 Node 级 bind，再从 Condition 栈顶向外查找。对应 Java:
    /// `NodeComponent#getBindData`。
    fn get_bind_data(&self, ctx: &CmpContext, key: &str) -> Option<String> {
        ctx.bind_data(key).map(ToOwned::to_owned)
    }

    /// 将指定 key 的绑定数据解析为 JSON 数组。
    ///
    /// 对应 Java: `NodeComponent#getBindDataList`。
    fn get_bind_data_list(&self, ctx: &CmpContext, key: &str) -> Option<Vec<Value>> {
        serde_json::from_str(ctx.bind_data(key)?).ok()
    }

    /// 使用属性表达式读取 serde JSON 上下文值。
    ///
    /// Java 可以反射任意 Bean；Rust 按既定技术映射仅对 `serde_json::Value`
    /// 上下文执行安全属性访问。对应 Java: `NodeComponent#getContextValue`。
    fn get_context_value(&self, ctx: &CmpContext, expression: &str) -> Option<Value> {
        let context_list = json_context_list(ctx);
        LiteflowContextRegexMatcher::search_context(&context_list, expression)
    }

    /// 使用 setter/属性表达式更新 serde JSON 上下文值。
    ///
    /// 成功时把修改后的 JSON 对象重新写回 Slot，并返回 `true`。对应 Java:
    /// `NodeComponent#setContextValue`。
    fn set_context_value(
        &self,
        ctx: &CmpContext,
        method_expression: &str,
        values: &[Value],
    ) -> bool {
        let mut context_list = json_context_list(ctx);
        if !LiteflowContextRegexMatcher::search_and_set_context(
            &mut context_list,
            method_expression,
            values,
        ) {
            return false;
        }
        for (context_name, context_value) in context_list {
            ctx.inner
                .insert_context_bean(context_name, Arc::new(context_value));
        }
        true
    }

    /// 返回最内层循环下标。对应 Java: `NodeComponent#getLoopIndex`。
    fn get_loop_index(&self, ctx: &CmpContext) -> Option<usize> {
        ctx.frame.loop_index_at(0)
    }

    /// 返回上一层循环下标。对应 Java: `NodeComponent#getPreLoopIndex`。
    fn get_pre_loop_index(&self, ctx: &CmpContext) -> Option<usize> {
        ctx.frame.loop_index_at(1)
    }

    /// 返回向外第 `n` 层循环下标。
    ///
    /// `n=0` 表示当前层。对应 Java: `NodeComponent#getPreNLoopIndex`。
    fn get_pre_n_loop_index(&self, ctx: &CmpContext, n: usize) -> Option<usize> {
        ctx.frame.loop_index_at(n)
    }

    /// 返回最内层循环对象。对应 Java: `NodeComponent#getCurrLoopObj`。
    fn get_curr_loop_obj(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.frame.loop_object_at(0).cloned()
    }

    /// 返回上一层循环对象。对应 Java: `NodeComponent#getPreLoopObj`。
    fn get_pre_loop_obj(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.frame.loop_object_at(1).cloned()
    }

    /// 返回向外第 `n` 层循环对象。
    ///
    /// `n=0` 表示当前层。对应 Java: `NodeComponent#getPreNLoopObj`。
    fn get_pre_n_loop_obj(&self, ctx: &CmpContext, n: usize) -> Option<Value> {
        ctx.frame.loop_object_at(n).cloned()
    }

    /// 设置当前组件步骤的自定义数据。
    ///
    /// 对应 Java: `NodeComponent#setStepData`。
    fn set_step_data(&self, ctx: &CmpContext, step_data: Value) {
        ctx.set_step_data(step_data);
    }

    /// 返回当前节点最近一次成功处理结果。
    ///
    /// 默认组件未执行或尚未产生结果时返回 `None`。对应 Java:
    /// `NodeComponent#getItemResultMetaValue`。
    fn get_item_result_meta_value(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.frame.get_node_item_result(ctx.node.display())
    }

    /// 返回当前 Chain 的任务局部运行 ID。
    ///
    /// Chain 尚未进入执行主干时返回 `None`。对应 Java:
    /// `NodeComponent#getCurrChainRuntimeId`。
    fn get_curr_chain_runtime_id(&self, ctx: &CmpContext) -> Option<u64> {
        ctx.frame.runtime_id()
    }
}

/// 提取 Slot 中可以通过 serde 安全访问的具名 JSON 上下文。
fn json_context_list(ctx: &CmpContext) -> Vec<(String, Value)> {
    ctx.inner
        .get_context_bean_list()
        .into_iter()
        .filter_map(|(context_name, context_bean)| {
            Arc::downcast::<Value>(context_bean)
                .ok()
                .map(|context_value| (context_name, (*context_value).clone()))
        })
        .collect()
}
