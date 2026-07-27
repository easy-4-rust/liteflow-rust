//! 对应 Java 类：`com.yomahub.liteflow.flow.element.FallbackNode`。
//!
//! Java 在 EL 构建期为尚未注册的节点创建代理，并在执行期依据该节点所处
//! Condition 的位置选择 COMMON / BOOLEAN / SWITCH / FOR / ITERATOR 降级组件。
//! Rust 构建器会把位置推导出的 `NodeTypeEnum` 显式传给本对象，从而避免
//! 运行期向上反射 Condition，同时保留“执行前再次寻找原节点”的动态语义。

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::enums::NodeTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::node::Node;
use crate::flow::executor::NodeExecutor;
use crate::slot::{CmpContext, Ctx};

/// 缺失节点的降级代理。
///
/// 对应 Java: `FallbackNode`。节点表和降级表均由所属 `FlowBus` 共享；
/// 因此链构建完成后再注册原节点，首次执行时仍会优先使用原节点。
pub struct FallbackNode {
    expected_node_id: String,
    expected_node_type: NodeTypeEnum,
    nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>>,
    fallback_nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>>,
}

impl FallbackNode {
    /// 创建降级代理。
    ///
    /// 对应 Java: `FallbackNode#FallbackNode(String expectedNodeId)`，额外接收由
    /// Rust EL 构建器静态推导的节点类型。
    pub fn new(
        expected_node_id: impl Into<String>,
        expected_node_type: NodeTypeEnum,
        nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>>,
        fallback_nodes: Arc<DashMap<String, Arc<dyn NodeComponent>>>,
    ) -> Self {
        Self {
            expected_node_id: expected_node_id.into(),
            expected_node_type: normalize_fallback_type(expected_node_type),
            nodes,
            fallback_nodes,
        }
    }

    /// 原节点 id。对应 Java: `FallbackNode#getExpectedNodeId`。
    pub fn expected_node_id(&self) -> &str {
        &self.expected_node_id
    }

    /// 返回原节点 id 的 Java 命名入口。
    ///
    /// 对应 Java: `FallbackNode#getExpectedNodeId`。
    #[must_use]
    pub fn get_expected_node_id(&self) -> &str {
        self.expected_node_id()
    }

    /// 修改代理等待的原节点 id。
    ///
    /// 后续解析会立即使用新 id 重查运行时节点表，不缓存旧解析结果。
    /// - `expected_node_id`: 新的原节点 id。
    ///
    /// 对应 Java: `FallbackNode#setExpectedNodeId`。
    pub fn set_expected_node_id(&mut self, expected_node_id: impl Into<String>) {
        self.expected_node_id = expected_node_id.into();
    }

    /// 当前位置需要的降级组件类型。
    pub fn expected_node_type(&self) -> NodeTypeEnum {
        self.expected_node_type
    }

    /// 通过一个真实 `Node` 执行最终解析到的原节点或降级组件。
    ///
    /// 该入口保留节点执行器、重试、切面、步骤记录、结果缓存与回滚登记；正常
    /// FlowBus 路径仍由外层 Node 调度本组件，因此不会重复触发生命周期。
    ///
    /// - `context`: 当前 Slot、节点元数据与任务 Frame。
    /// - 返回：最终节点的真实执行结果。
    ///
    /// 对应 Java: `FallbackNode#execute`。
    pub async fn execute(&self, context: &CmpContext) -> LFResult<Value> {
        let component = self.resolve(context.chain_id())?;
        let mut node = Node::new(context.node.clone(), component);
        node.set_curr_chain_id(context.chain_id());
        node.execute(&Ctx::new(Arc::clone(&context.inner)), &context.frame)
            .await
    }

    /// 返回最终节点最近一次成功执行结果。
    ///
    /// 未执行或尚无可解析组件时返回 `None`，对应 Java 的 `null`。
    /// 对应 Java: `FallbackNode#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, context: &CmpContext) -> Option<Value> {
        self.resolve_without_context()
            .and_then(|component| component.get_item_result_meta_value(context))
    }

    /// 加载最终组件并判断当前节点是否允许执行。
    ///
    /// 与 Java 一样，访问判断可能先于正式执行，因此缺少降级组件时立即返回错误。
    /// 对应 Java: `FallbackNode#isAccess`。
    pub fn is_access(&self, context: &CmpContext) -> LFResult<bool> {
        Ok(self.resolve(context.chain_id())?.is_access(context))
    }

    /// 返回当前动态解析到的真实组件。
    ///
    /// 未注册原节点且未配置对应类型的降级组件时返回 `None`。对应 Java:
    /// `FallbackNode#getInstance`。
    #[must_use]
    pub fn get_instance(&self) -> Option<Arc<dyn NodeComponent>> {
        self.resolve_without_context()
    }

    /// 返回最终组件的节点 id。
    ///
    /// Rust 组件初始化器直接保存节点 id；尚不能解析组件时返回 `None`，对应
    /// Java 的 `null`。对应 Java: `FallbackNode#getId`。
    #[must_use]
    pub fn get_id(&self) -> Option<String> {
        self.resolve_without_context()
            .map(|component| component.get_node_id().to_string())
            .filter(|node_id| !node_id.is_empty())
    }

    /// 返回当前代理本身。
    ///
    /// Java 明确规定代理节点不复制而直接返回 `this`；Rust 用共享引用表达同一
    /// 对象身份。对应 Java: `FallbackNode#clone`。
    #[must_use]
    pub fn clone(&self) -> &Self {
        self
    }

    /// 返回固定的 FALLBACK 节点类型。
    ///
    /// 对应 Java: `FallbackNode#getType`。
    #[must_use]
    pub fn get_type(&self) -> NodeTypeEnum {
        NodeTypeEnum::Fallback
    }

    /// 先寻找执行期已经注册的原节点，否则按类型寻找降级节点。
    ///
    /// 对应 Java: `FallbackNode#execute` 与 `#loadFallBackNode`。
    fn resolve(&self, chain_id: &str) -> LFResult<Arc<dyn NodeComponent>> {
        if let Some(component) = self.nodes.get(&self.expected_node_id) {
            return Ok(component.clone());
        }
        if let Some(component) = self.fallback_nodes.get(self.expected_node_type.get_code()) {
            return Ok(component.clone());
        }
        Err(LiteflowError::FallbackCmpNotFound(format!(
            "No fallback component found for [{}] in chain[{}].",
            self.expected_node_id, chain_id,
        )))
    }

    fn resolve_without_context(&self) -> Option<Arc<dyn NodeComponent>> {
        self.nodes
            .get(&self.expected_node_id)
            .map(|component| component.clone())
            .or_else(|| {
                self.fallback_nodes
                    .get(self.expected_node_type.get_code())
                    .map(|component| component.clone())
            })
    }
}

#[async_trait]
impl NodeComponent for FallbackNode {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.resolve(ctx.chain_id())?.process(ctx).await
    }

    async fn before_process(&self, ctx: &CmpContext) -> LFResult<()> {
        self.resolve(ctx.chain_id())?.before_process(ctx).await
    }

    async fn on_success(&self, ctx: &CmpContext) -> LFResult<()> {
        self.resolve(ctx.chain_id())?.on_success(ctx).await
    }

    async fn after_process(&self, ctx: &CmpContext) {
        if let Ok(component) = self.resolve(ctx.chain_id()) {
            component.after_process(ctx).await;
        }
    }

    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        if let Ok(component) = self.resolve(ctx.chain_id()) {
            component.on_error(ctx, error).await;
        }
    }

    fn is_access(&self, ctx: &CmpContext) -> bool {
        self.resolve_without_context()
            .map(|component| component.is_access(ctx))
            // NodeComponent::is_access 不能返回 Result；缺少降级组件时放行，
            // 由 before_process/process 返回 FallbackCmpNotFound。
            .unwrap_or(true)
    }

    fn is_continue_on_error(&self) -> bool {
        self.resolve_without_context()
            .map(|component| component.is_continue_on_error())
            .unwrap_or(false)
    }

    fn is_rollback(&self) -> bool {
        self.resolve_without_context()
            .map(|component| component.is_rollback())
            .unwrap_or(false)
    }

    async fn rollback(&self, ctx: &CmpContext) -> LFResult<()> {
        self.resolve(ctx.chain_id())?.rollback(ctx).await
    }

    fn retry_count(&self) -> usize {
        self.resolve_without_context()
            .map(|component| component.retry_count())
            .unwrap_or(0)
    }

    fn is_retry_for(&self, error: &LiteflowError) -> bool {
        self.resolve_without_context()
            .map(|component| component.is_retry_for(error))
            .unwrap_or(false)
    }

    fn node_executor(&self) -> Option<Arc<dyn NodeExecutor>> {
        self.resolve_without_context()
            .and_then(|component| component.node_executor())
    }
}

/// Java 的 IF/WHILE/BREAK 都落到 BOOLEAN fallback；脚本节点缺失时也按其
/// 运行结果类型归一化到对应的非脚本 fallback。
pub(crate) fn normalize_fallback_type(node_type: NodeTypeEnum) -> NodeTypeEnum {
    match node_type {
        NodeTypeEnum::If
        | NodeTypeEnum::While
        | NodeTypeEnum::Break
        | NodeTypeEnum::BooleanScript
        | NodeTypeEnum::IfScript
        | NodeTypeEnum::WhileScript
        | NodeTypeEnum::BreakScript => NodeTypeEnum::Boolean,
        NodeTypeEnum::SwitchScript => NodeTypeEnum::Switch,
        NodeTypeEnum::ForScript => NodeTypeEnum::For,
        NodeTypeEnum::Script | NodeTypeEnum::Fallback => NodeTypeEnum::Common,
        other => other,
    }
}
