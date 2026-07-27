//! 对应 flow.element.Chain：按序执行 conditionList；
//! 决策表链路持有 routeItem（executeRoute 语义，2.12+）。

use crate::common::ChainConstant;
use crate::enums::{ChainExecuteModeEnum, ExecuteableTypeEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::condition::expect_bool;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// 默认路由命名空间，对应 Java `ChainConstant.DEFAULT_NAMESPACE`。
pub const DEFAULT_NAMESPACE: &str = ChainConstant::DEFAULT_NAMESPACE;

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

/// 流程链定义，负责保存主体 Condition、决策路由和编译元数据。
///
/// 对应 Java: `com.yomahub.liteflow.flow.element.Chain`。
#[derive(Clone)]
pub struct Chain {
    pub id: String,
    pub namespace: String,
    /// 构建该链的 EL 原文（2.16：getEl/getElMd5，execute2RespWithEL 缓存索引用）
    el: Option<String>,
    el_md5: Option<String>,
    /// 决策表链路的 route EL（对应 routeItem）
    route_item: Option<Arc<dyn Executable>>,
    /// Chain 层级执行器构建器名称。
    thread_pool_executor_class: Option<String>,
    condition_list: Vec<Arc<dyn Executable>>,
    route_el: Option<String>,
    extends_chain_id: Option<String>,
    is_abstract: bool,
    is_compiled: bool,
}

impl Chain {
    /// 创建已经持有执行主体的 Chain。
    ///
    /// 参数 `id`、`condition_list` 对应 Java 构造器的 `chainName`、
    /// `conditionList`；非空主体视为已经编译。
    pub fn new(id: impl Into<String>, condition_list: Vec<Arc<dyn Executable>>) -> Self {
        let is_compiled = !condition_list.is_empty();
        Self {
            id: id.into(),
            namespace: DEFAULT_NAMESPACE.to_string(),
            el: None,
            el_md5: None,
            route_item: None,
            thread_pool_executor_class: None,
            condition_list,
            route_el: None,
            extends_chain_id: None,
            is_abstract: false,
            is_compiled,
        }
    }

    /// 返回主体 Condition 快照。对应 Java: `Chain#getConditionList`。
    #[must_use]
    pub fn get_condition_list(&self) -> Vec<Arc<dyn Executable>> {
        self.condition_list.clone()
    }

    /// 替换主体 Condition。
    ///
    /// 参数 `condition_list` 对应 Java 同名参数。对应 Java:
    /// `Chain#setConditionList`。
    pub fn set_condition_list(&mut self, condition_list: Vec<Arc<dyn Executable>>) {
        self.is_compiled = !condition_list.is_empty();
        self.condition_list = condition_list;
    }

    /// 返回 Chain ID。
    ///
    /// 已废弃 Java 名称的 Rust 对等入口。对应 Java: `Chain#getChainName`。
    #[deprecated(note = "请使用 get_chain_id")]
    #[must_use]
    pub fn get_chain_name(&self) -> &str {
        self.get_chain_id()
    }

    /// 设置 Chain ID。
    ///
    /// 已废弃 Java 名称的 Rust 对等入口。对应 Java: `Chain#setChainName`。
    #[deprecated(note = "请使用 set_chain_id")]
    pub fn set_chain_name(&mut self, chain_name: impl Into<String>) {
        self.set_chain_id(chain_name);
    }

    /// 返回 Chain ID。对应 Java: `Chain#getChainId`。
    #[must_use]
    pub fn get_chain_id(&self) -> &str {
        &self.id
    }

    /// 设置 Chain ID。
    ///
    /// 参数 `chain_id` 对应 Java 同名参数。对应 Java: `Chain#setChainId`。
    pub fn set_chain_id(&mut self, chain_id: impl Into<String>) {
        self.id = chain_id.into();
    }

    /// 设置构建该链的 EL 原文。对应 Java: `Chain#setEl`。
    pub fn set_el(&mut self, el: impl Into<String>) {
        self.el = Some(el.into());
    }

    /// 返回构建该链的 EL 原文。对应 Java: `Chain#getEl`。
    #[must_use]
    pub fn get_el(&self) -> Option<&str> {
        self.el.as_deref()
    }

    /// 返回 EL 的 MD5 缓存键。对应 Java: `Chain#getElMd5`。
    #[must_use]
    pub fn get_el_md5(&self) -> Option<&str> {
        self.el_md5.as_deref()
    }

    /// 设置 EL 的 MD5 缓存键。
    ///
    /// 参数 `el_md5` 对应 Java 同名参数。对应 Java: `Chain#setElMd5`。
    pub fn set_el_md5(&mut self, el_md5: impl Into<String>) {
        self.el_md5 = Some(el_md5.into());
    }

    /// 以链式方式设置命名空间。
    ///
    /// 这是 Rust 构建器便利入口，字段语义对应 Java `Chain#setNamespace`。
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.set_namespace(namespace);
        self
    }

    /// 设置决策路由执行项。对应 Java: `Chain#setRouteItem`。
    pub fn set_route_item(&mut self, route: Arc<dyn Executable>) {
        self.route_item = Some(route);
    }

    /// 返回决策路由执行项。对应 Java: `Chain#getRouteItem`。
    #[must_use]
    pub fn get_route_item(&self) -> Option<&Arc<dyn Executable>> {
        self.route_item.as_ref()
    }

    /// 返回 Chain 层级执行器构建器名称。
    ///
    /// 对应 Java: `Chain#getThreadPoolExecutorClass`。
    #[must_use]
    pub fn get_thread_pool_executor_class(&self) -> Option<&str> {
        self.thread_pool_executor_class.as_deref()
    }

    /// 设置 Chain 层级执行器构建器名称。
    ///
    /// 对应 Java: `Chain#setThreadPoolExecutorClass`。
    pub fn set_thread_pool_executor_class(
        &mut self,
        thread_pool_executor_class: impl Into<String>,
    ) {
        self.thread_pool_executor_class = Some(thread_pool_executor_class.into());
    }

    /// 以链式方式设置 Chain 层级执行器构建器名称。
    #[must_use]
    pub fn with_thread_pool_executor_class(
        mut self,
        thread_pool_executor_class: impl Into<String>,
    ) -> Self {
        self.set_thread_pool_executor_class(thread_pool_executor_class);
        self
    }

    /// 返回当前 Chain 是否已经编译。对应 Java: `Chain#isCompiled`。
    #[must_use]
    pub fn is_compiled(&self) -> bool {
        self.is_compiled
    }

    /// 设置当前 Chain 的编译状态。
    ///
    /// 参数 `compiled` 对应 Java 同名参数。对应 Java: `Chain#setCompiled`。
    pub fn set_compiled(&mut self, compiled: bool) {
        self.is_compiled = compiled;
    }

    /// 返回 Chain 命名空间。对应 Java: `Chain#getNamespace`。
    #[must_use]
    pub fn get_namespace(&self) -> &str {
        &self.namespace
    }

    /// 设置 Chain 命名空间。
    ///
    /// 参数 `namespace` 为空时仍按 Java setter 原样保存。
    /// 对应 Java: `Chain#setNamespace`。
    pub fn set_namespace(&mut self, namespace: impl Into<String>) {
        self.namespace = namespace.into();
    }

    /// 返回当前任务的 Chain 运行标识。
    ///
    /// 参数 `frame` 映射 Java `runtimeIdTL`。对应 Java: `Chain#getRuntimeId`。
    #[must_use]
    pub fn get_runtime_id(&self, frame: &Frame) -> Option<u64> {
        frame.runtime_id()
    }

    /// 返回决策路由 EL。对应 Java: `Chain#getRouteEl`。
    #[must_use]
    pub fn get_route_el(&self) -> Option<&str> {
        self.route_el.as_deref()
    }

    /// 设置决策路由 EL。
    ///
    /// 参数 `route_el` 对应 Java 同名参数。对应 Java: `Chain#setRouteEl`。
    pub fn set_route_el(&mut self, route_el: impl Into<String>) {
        self.route_el = Some(route_el.into());
    }

    /// 返回当前 Chain 是否为抽象链。对应 Java: `Chain#isAbstract`。
    #[must_use]
    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    /// 设置当前 Chain 是否为抽象链。
    ///
    /// 参数 `is_abstract` 对应 Java `anAbstract`。对应 Java:
    /// `Chain#setAbstract`。
    pub fn set_abstract(&mut self, is_abstract: bool) {
        self.is_abstract = is_abstract;
    }

    /// 返回继承的父 Chain ID。对应 Java: `Chain#getExtendsChainId`。
    #[must_use]
    pub fn get_extends_chain_id(&self) -> Option<&str> {
        self.extends_chain_id.as_deref()
    }

    /// 设置继承的父 Chain ID。
    ///
    /// 参数 `extends_chain_id` 对应 Java 同名参数。对应 Java:
    /// `Chain#setExtendsChainId`。
    pub fn set_extends_chain_id(&mut self, extends_chain_id: impl Into<String>) {
        self.extends_chain_id = Some(extends_chain_id.into());
    }

    /// 返回可执行对象类型。对应 Java: `Chain#getExecuteType`。
    #[must_use]
    pub fn get_execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Chain
    }

    /// 设置可执行对象 ID。对应 Java: `Chain#setId`。
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.set_chain_id(id);
    }

    /// 返回可执行对象 ID。对应 Java: `Chain#getId`。
    #[must_use]
    pub fn get_id(&self) -> &str {
        self.get_chain_id()
    }

    /// Chain 不保存标签；保留 Java 接口的空实现。对应 Java: `Chain#setTag`。
    pub fn set_tag(&mut self, _tag: impl Into<String>) {}

    /// Chain 不保存标签，始终返回 `None`。对应 Java: `Chain#getTag`。
    #[must_use]
    pub fn get_tag(&self) -> Option<&str> {
        None
    }

    /// Chain.execute(slotIndex)：按序执行 conditionList
    pub async fn execute(&self, ctx: &Ctx) -> LFResult<Value> {
        self.execute_with_frame(ctx, &Frame::root()).await
    }

    /// 以指定执行路径帧执行（ChainBindWrapperCondition 下传 bind/loop 栈用，
    /// 对齐 Java 子链共享 Slot 的 conditionStack / loop 栈语义）
    pub async fn execute_with_frame(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let runtime_id = NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed);
        let frame = frame
            .with_runtime_id(runtime_id)
            .with_current_chain_id(self.id.clone())
            .with_chain_thread_pool(self.thread_pool_executor_class.as_deref());

        // Rust 的 Slot 在 DataBus 租约创建时已经绑定主 Chain ID；子链共享该值，
        // 与 Java setChainId 仅在元数据为空时写入的行为一致。
        for condition in &self.condition_list {
            if let Err(error) = condition.execute(ctx, &frame).await {
                // ChainEnd 是正常的主动结束信号，不写入异常槽。
                if !matches!(error, LiteflowError::ChainEnd(_)) {
                    ctx.set_exception(&error.to_string());
                }
                return Err(error);
            }
        }
        Ok(Value::Null)
    }

    /// Chain.executeRoute(slotIndex)：求 route EL 的布尔结果
    pub async fn execute_route(&self, ctx: &Ctx) -> LFResult<bool> {
        let route = self
            .route_item
            .as_ref()
            .ok_or_else(|| LiteflowError::Custom(format!("chain[{}] has no route", self.id)))?;
        let v = route.execute(ctx, &Frame::root()).await?;
        let route_result = expect_bool(route.id(), &v)?;
        ctx.inner.set_route_result(route_result);
        Ok(route_result)
    }

    /// 按 Chain 执行模式运行主体或决策路由。
    ///
    /// 对应 Java `FlowExecutor#doExecute(..., ChainExecuteModeEnum)`：
    /// BODY 进入 `Chain#execute`，ROUTE 进入 `Chain#executeRoute`。
    pub async fn execute_mode(&self, ctx: &Ctx, mode: ChainExecuteModeEnum) -> LFResult<Value> {
        match mode {
            ChainExecuteModeEnum::Body => self.execute(ctx).await,
            ChainExecuteModeEnum::Route => self.execute_route(ctx).await.map(Value::Bool),
        }
    }
}

#[async_trait::async_trait]
impl Executable for Chain {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        self.execute_with_frame(ctx, frame).await
    }

    fn execute_type(&self) -> ExecuteableTypeEnum {
        self.get_execute_type()
    }

    fn collect_node_ids(&self) -> Vec<String> {
        self.condition_list
            .iter()
            .flat_map(|condition| condition.collect_node_ids())
            .collect()
    }

    fn id(&self) -> &str {
        &self.id
    }
}
