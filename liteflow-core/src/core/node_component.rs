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
use crate::flow::element::NodeHooks;
use crate::flow::executor::NodeExecutor;
use crate::flow::liteflow_response::LiteflowResponse;
use crate::slot::{CmpContext, Frame, Slot};
use crate::util::LiteflowContextRegexMatcher;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::{Any, TypeId, type_name};
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[async_trait]
pub trait NodeComponent: Send + Sync + 'static {
    /// process() / processIf() / processSwitch() / processFor() / processIterator()
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>;

    /// 执行组件的完整处理生命周期。
    ///
    /// 参数 `ctx` 是 Java 通过 `refNode` 与 `slotIndex` 隐式取得的执行上下文；
    /// `result_frame` 是当前任务的结果写回目标，避免写入为异步分支隔离而深复制
    /// 的上下文 Frame；`hooks` 承载 `CmpAroundAspectHolder` 的执行期快照。方法执行：
    /// 全局前置切面 → `before_process` → `process` → `on_success`；
    /// 失败时执行 `on_error` 和全局错误切面；最后始终执行组件及全局
    /// `after_process`。有返回值组件会在 `on_success` 前写入当前 Frame，保持
    /// Java Boolean/Switch/For/Iterator 组件先写 Slot 结果再回调的顺序。
    ///
    /// CmpStep、耗时监控和重试仍由 Rust 的 `Node`/`NodeExecutor` 外层负责。
    /// 对应 Java: `com.yomahub.liteflow.core.NodeComponent#execute`。
    async fn execute(
        &self,
        ctx: &CmpContext,
        result_frame: &Frame,
        hooks: &NodeHooks,
    ) -> Result<Value, LiteflowError> {
        for aspect in &hooks.aspects {
            aspect.before_process(ctx);
        }

        let result = match self.before_process(ctx).await {
            Ok(()) => match self.process(ctx).await {
                Ok(value) => {
                    // Java 的有返回值组件在 onSuccess 之前把结果写入 Slot。
                    result_frame.set_node_item_result(ctx.node_id().to_string(), value.clone());
                    self.on_success(ctx).await.map(|()| value)
                }
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };

        match &result {
            Ok(_) => {
                for aspect in &hooks.aspects {
                    aspect.on_success(ctx);
                }
            }
            Err(error) => {
                // Java 会忽略 onError 自身的异常并继续传播主要异常；Rust 钩子
                // 不返回 Result，因此天然保持该错误优先级。
                self.on_error(ctx, error).await;
                for aspect in &hooks.aspects {
                    aspect.on_error(ctx, error);
                }
            }
        }

        self.after_process(ctx).await;
        for aspect in &hooks.aspects {
            aspect.after_process(ctx);
        }

        result
    }

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
    /// 异步判断组件是否允许执行。
    ///
    /// 普通组件默认委托同步 `is_access`；声明式组件可调用异步业务方法。对应 Java:
    /// `NodeComponent#isAccess` 经 ByteBuddy InvocationHandler 动态分派。
    async fn is_access_async(&self, ctx: &CmpContext) -> Result<bool, LiteflowError> {
        Ok(self.is_access(ctx))
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
    /// 异步判断当前错误是否继续执行。
    ///
    /// 普通组件保持同步兼容，声明式组件可读取当前上下文。对应 Java:
    /// `NodeComponent#isContinueOnError`。
    async fn is_continue_on_error_async(&self, ctx: &CmpContext) -> Result<bool, LiteflowError> {
        Ok(self.is_continue_on_error_with_context(ctx))
    }
    /// 是否需要失败补偿。
    ///
    /// Java 构造器通过反射判断组件是否覆盖 `rollback()`；Rust trait 无法在运行时
    /// 可靠判断默认方法是否被覆盖，因此显式返回该能力。后续 `liteflow-derive`
    /// 会在声明了 rollback 方法时自动生成此标记。该能力标记对应 Java
    /// `NodeComponent#setRollback` 写入的真实状态，不提供执行后可变空 setter。
    fn is_rollback(&self) -> bool {
        false
    }
    /// Rollbackable.rollback()。
    ///
    /// 对应 Java: `com.yomahub.liteflow.core.NodeComponent#rollback`。
    async fn rollback(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }

    /// 执行当前组件的回滚入口。
    ///
    /// 参数 `ctx` 显式承载 Java ThreadLocal 中的 Slot 与 RefNode。Rust 将
    /// `doRollback` 中的重复检查和 CmpStep 记录放在 `Ctx`/`Node` 外层，本方法
    /// 负责调用真实 `rollback` 实现；所有运行时回滚路径均经由此入口。
    /// 对应 Java: `com.yomahub.liteflow.core.NodeComponent#doRollback`。
    async fn do_rollback(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        self.rollback(ctx).await
    }
    /// getName()
    fn name(&self) -> &str {
        ""
    }
    /// 异步解析当前执行的组件显示名覆盖。
    ///
    /// 普通组件返回 `None` 并继续使用 `name`；声明式组件可通过
    /// `@LiteflowMethod(GET_DISPLAY_NAME)` 返回动态名称。对应 Java:
    /// `NodeComponent#getDisplayName` 的代理拦截。
    async fn display_name_async(&self, _ctx: &CmpContext) -> Result<Option<String>, LiteflowError> {
        Ok(None)
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
    /// getRetryForExceptions() 语义：判断抛出的异常是否命中组件声明的可重试异常范围。
    ///
    /// Java 默认值为 `Exception.class`，因此未声明过滤器时接受所有普通执行错误；
    /// `#[liteflow_retry]` 会为显式列表生成更精确的谓词。
    fn is_retry_for(&self, _e: &LiteflowError) -> bool {
        true
    }
    /// getNodeExecutorClass()：指定自定义节点执行器；None 表示使用 DefaultNodeExecutor
    /// （Java 返回 Class 由 NodeExecutorHelper 经 DI 容器实例化并缓存，
    /// Rust 端无 DI 容器，直接提供 Arc 实例）
    fn node_executor(&self) -> Option<Arc<dyn NodeExecutor>> {
        None
    }
    /// 异步解析当前声明式组件指定的 Java 节点执行器类名。
    ///
    /// 普通组件返回 `None`；声明式组件把 Java `Class` 映射为注册表类名。对应
    /// Java: `NodeComponent#getNodeExecutorClass`。
    async fn node_executor_class_async(
        &self,
        _ctx: &CmpContext,
    ) -> Result<Option<String>, LiteflowError> {
        Ok(None)
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
    /// 异步判断当前组件是否请求结束链路。
    ///
    /// 普通组件默认委托同步 `is_end`；声明式组件由 `@LiteflowMethod(IS_END)`
    /// 返回结果。对应 Java: `NodeComponent#isEnd`。
    async fn is_end_async(&self, ctx: &CmpContext) -> Result<bool, LiteflowError> {
        Ok(self.is_end(ctx))
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

    /// 按 Rust 运行时类型返回第一个匹配的上下文 Bean。
    ///
    /// - `ctx`: 当前组件执行上下文。
    /// - 返回：与 `T` 类型一致的首个共享 Bean；未找到时返回 `None`。
    ///
    /// Java 通过 `Class<T>` 参数检索，Rust 使用 `Any` 的安全向下转型表达相同
    /// 能力。对应 Java: `NodeComponent#getContextBean(Class<T>)`。
    fn get_context_bean_by_type<T: Any + Send + Sync>(&self, ctx: &CmpContext) -> Option<Arc<T>>
    where
        Self: Sized,
    {
        ctx.inner.get_context_bean_by_type::<T>()
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

    /// 将当前节点组件数据反序列化为指定 Rust 类型。
    ///
    /// - `ctx`: 当前组件执行上下文。
    /// - 返回：空数据返回 `Ok(None)`；`String` 保留原文，其他类型通过 serde
    ///   反序列化；转换失败返回 `ObjectConvertException` 对应错误。
    ///
    /// 对应 Java: `NodeComponent#getCmpData(Class<T>)`。
    fn get_cmp_data_as<T>(&self, ctx: &CmpContext) -> Result<Option<T>, LiteflowError>
    where
        Self: Sized,
        T: DeserializeOwned + Any,
    {
        ctx.cmp_data()
            .map(deserialize_component_text::<T>)
            .transpose()
    }

    /// 将当前节点组件数据解析为 JSON 数组。
    ///
    /// 非数组或非法 JSON 返回 `None`，对应 Java:
    /// `NodeComponent#getCmpDataList` 的 serde 映射。
    fn get_cmp_data_list(&self, ctx: &CmpContext) -> Option<Vec<Value>> {
        serde_json::from_str(ctx.cmp_data()?).ok()
    }

    /// 将当前节点组件数据反序列化为指定类型列表。
    ///
    /// 空数据返回 `Ok(None)`；非法 JSON 或元素类型不兼容时返回对象转换错误。
    /// 对应 Java: `NodeComponent#getCmpDataList(Class<T>)`。
    fn get_cmp_data_list_as<T>(&self, ctx: &CmpContext) -> Result<Option<Vec<T>>, LiteflowError>
    where
        Self: Sized,
        T: DeserializeOwned,
    {
        ctx.cmp_data()
            .map(|data| {
                serde_json::from_str(data).map_err(|error| {
                    LiteflowError::ObjectConvert(format!(
                        "component data cannot convert to List<{}>: {error}",
                        type_name::<T>()
                    ))
                })
            })
            .transpose()
    }

    /// 返回指定 key 的绑定数据。
    ///
    /// 先查 Node 级 bind，再从 Condition 栈顶向外查找。对应 Java:
    /// `NodeComponent#getBindData`。
    fn get_bind_data(&self, ctx: &CmpContext, key: &str) -> Option<String> {
        let bind_data = ctx.bind_data(key)?;
        let expression = context_search_expression(bind_data)?;
        if expression == bind_data {
            return Some(bind_data.to_string());
        }
        self.get_context_value(ctx, expression)
            .and_then(value_to_component_text)
    }

    /// 将指定 bind 数据转换为目标 Rust 类型。
    ///
    /// Node 级 bind 优先于 Condition bind；`${context.path}` 会先从上下文 Bean
    /// 求值，再通过 serde 转换为 `T`。普通字符串在 `T=String` 时保持原文，其他
    /// 类型按 JSON 解析。未找到时返回 `Ok(None)`，类型不兼容返回对象转换错误。
    ///
    /// 对应 Java: `NodeComponent#getBindData(String, Class<T>)`。
    fn get_bind_data_as<T>(&self, ctx: &CmpContext, key: &str) -> Result<Option<T>, LiteflowError>
    where
        Self: Sized,
        T: DeserializeOwned + Any,
    {
        let Some(bind_data) = ctx.bind_data(key) else {
            return Ok(None);
        };
        let Some(expression) = context_search_expression(bind_data) else {
            return Ok(None);
        };
        if expression != bind_data {
            return self
                .get_context_value(ctx, expression)
                .map(deserialize_component_value::<T>)
                .transpose();
        }
        deserialize_component_text(bind_data).map(Some)
    }

    /// 将指定 key 的绑定数据解析为 JSON 数组。
    ///
    /// 对应 Java: `NodeComponent#getBindDataList`。
    fn get_bind_data_list(&self, ctx: &CmpContext, key: &str) -> Option<Vec<Value>> {
        serde_json::from_str(ctx.bind_data(key)?).ok()
    }

    /// 将指定 bind 数据反序列化为目标类型列表。
    ///
    /// 与 Java `getBindDataList` 一致，本入口解析 bind 本身的 JSON 数组，不把
    /// `${...}` 解释为列表表达式。对应 Java:
    /// `NodeComponent#getBindDataList(String, Class<T>)`。
    fn get_bind_data_list_as<T>(
        &self,
        ctx: &CmpContext,
        key: &str,
    ) -> Result<Option<Vec<T>>, LiteflowError>
    where
        Self: Sized,
        T: DeserializeOwned,
    {
        ctx.bind_data(key)
            .map(|bind_data| {
                serde_json::from_str(bind_data).map_err(|error| {
                    LiteflowError::ObjectConvert(format!(
                        "bind data[{key}] cannot convert to List<{}>: {error}",
                        type_name::<T>()
                    ))
                })
            })
            .transpose()
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

    /// 在组件内部执行另一条链路，并把子链执行步骤合并回当前 Slot。
    ///
    /// - `ctx`: 当前组件执行上下文，用于继承请求编号、上下文 Bean 和父 Slot。
    /// - `chain_id`: 待调用的子链标识，对应 Java 参数 `chainId`。
    /// - `request_data`: 子链请求数据，对应 Java 参数 `requestData`。
    /// - 返回：子链完整响应；执行器尚未初始化时返回明确错误。
    ///
    /// Java 从组件 ThreadLocal 隐式取得 Slot；Rust 显式接收 `CmpContext`，避免
    /// 共享组件在并发任务间串用上下文。对应 Java:
    /// `NodeComponent#invoke2Resp(String, Object)`。
    async fn invoke2_resp(
        &self,
        ctx: &CmpContext,
        chain_id: &str,
        request_data: Value,
    ) -> Result<LiteflowResponse, LiteflowError> {
        self.invoke2_resp_with_slot(chain_id, request_data, self.get_slot(ctx))
            .await
    }

    /// 使用指定父 Slot 在组件内部执行另一条链路。
    ///
    /// - `chain_id`: 待调用的子链标识，对应 Java 参数 `chainId`。
    /// - `request_data`: 子链请求数据，对应 Java 参数 `requestData`。
    /// - `slot`: 继承请求编号、上下文 Bean 并接收子链步骤的父 Slot。
    /// - 返回：子链完整响应。
    ///
    /// 对应 Java: `NodeComponent#invoke2Resp(String, Object, Slot)`。
    async fn invoke2_resp_with_slot(
        &self,
        chain_id: &str,
        request_data: Value,
        slot: Arc<Slot>,
    ) -> Result<LiteflowResponse, LiteflowError> {
        let executor = crate::core::FlowExecutorHolder::load_instance()?;
        let response = executor
            .execute2_resp_with_rid(
                chain_id,
                request_data,
                slot.request_id.clone(),
                slot.get_context_bean_list(),
            )
            .await;

        // Java 将子链响应的执行步骤追加到父 Slot，外层响应因而保留完整调用轨迹。
        for step in response.get_execute_step_queue() {
            slot.add_step(step.clone());
        }
        Ok(response)
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

/// 提取 Java `${context.path}` 绑定表达式；普通 bind 原样返回。
fn context_search_expression(bind_data: &str) -> Option<&str> {
    let trimmed = bind_data.trim();
    if trimmed.starts_with("${") && trimmed.ends_with('}') {
        let expression = trimmed[2..trimmed.len() - 1].trim();
        return (!expression.is_empty()).then_some(expression);
    }
    Some(bind_data)
}

/// 把 serde 值转换为未类型化字符串入口的返回值。
fn value_to_component_text(value: Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value),
        Value::Null => None,
        value => serde_json::to_string(&value).ok(),
    }
}

/// 按 Java String/Object 原文分支或 serde JSON 分支转换组件文本。
fn deserialize_component_text<T>(data: &str) -> Result<T, LiteflowError>
where
    T: DeserializeOwned + Any,
{
    if TypeId::of::<T>() == TypeId::of::<String>() {
        let value: Box<dyn Any> = Box::new(data.to_string());
        return value.downcast::<T>().map(|value| *value).map_err(|_| {
            LiteflowError::ObjectConvert(format!(
                "component text cannot convert to {}",
                type_name::<T>()
            ))
        });
    }
    serde_json::from_str(data).map_err(|error| {
        LiteflowError::ObjectConvert(format!(
            "component text cannot convert to {}: {error}",
            type_name::<T>()
        ))
    })
}

/// 把上下文搜索得到的 serde 值转换为目标类型。
fn deserialize_component_value<T>(value: Value) -> Result<T, LiteflowError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|error| {
        LiteflowError::ObjectConvert(format!(
            "context value cannot convert to {}: {error}",
            type_name::<T>()
        ))
    })
}
