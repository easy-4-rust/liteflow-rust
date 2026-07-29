//! 组件初始化后的不可变委托对象。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::enums::NodeTypeEnum;
use crate::exception::LiteflowError;
use crate::flow::element::NodeHooks;
use crate::flow::executor::NodeExecutor;
use crate::property::LiteflowConfigGetter;
use crate::slot::{CmpContext, Frame};

/// 保存 Java `ComponentInitializer` 原本写入组件实例的节点元数据。
///
/// Rust 组件通常以 `Arc<dyn NodeComponent>` 共享，不能像 Java Bean 一样在注册后
/// 修改字段，因此用不可变委托保存 nodeId、type、name、默认重试与执行器。
/// `setNodeId/setName/setType` 映射为构造时一次写入；`setSelf/getSelf` 映射为
/// `inner` 委托，保留代理组件对生命周期入口的覆盖。
///
/// 对应 Java: `com.yomahub.liteflow.core.ComponentInitializer#initComponent`。
pub(crate) struct InitializedNodeComponent {
    inner: Arc<dyn NodeComponent>,
    node_id: String,
    node_type: NodeTypeEnum,
    node_type_explicit: bool,
    name: String,
    default_retry_count: Option<usize>,
    default_node_executor: Option<Arc<dyn NodeExecutor>>,
}

impl InitializedNodeComponent {
    /// 创建已经完成元数据注入的组件委托。
    pub(crate) fn new(
        inner: Arc<dyn NodeComponent>,
        node_id: String,
        node_type: NodeTypeEnum,
        node_type_explicit: bool,
        name: String,
        default_retry_count: Option<usize>,
        default_node_executor: Option<Arc<dyn NodeExecutor>>,
    ) -> Self {
        Self {
            inner,
            node_id,
            node_type,
            node_type_explicit,
            name,
            default_retry_count,
            default_node_executor,
        }
    }
}

#[async_trait]
impl NodeComponent for InitializedNodeComponent {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.inner.process(ctx).await
    }

    async fn execute(
        &self,
        ctx: &CmpContext,
        result_frame: &Frame,
        hooks: &NodeHooks,
    ) -> Result<Value, LiteflowError> {
        // 包装器只提供不可变元数据，不能截断内部组件对 Java execute 入口的覆盖。
        self.inner.execute(ctx, result_frame, hooks).await
    }

    async fn before_process(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        self.inner.before_process(ctx).await
    }

    async fn on_success(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        self.inner.on_success(ctx).await
    }

    async fn after_process(&self, ctx: &CmpContext) {
        self.inner.after_process(ctx).await;
    }

    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        self.inner.on_error(ctx, error).await;
    }

    fn is_access(&self, ctx: &CmpContext) -> bool {
        self.inner.is_access(ctx)
    }

    async fn is_access_async(&self, ctx: &CmpContext) -> Result<bool, LiteflowError> {
        self.inner.is_access_async(ctx).await
    }

    fn is_continue_on_error(&self) -> bool {
        self.inner.is_continue_on_error()
    }

    fn is_continue_on_error_with_context(&self, context: &CmpContext) -> bool {
        self.inner.is_continue_on_error_with_context(context)
    }

    async fn is_continue_on_error_async(
        &self,
        context: &CmpContext,
    ) -> Result<bool, LiteflowError> {
        self.inner.is_continue_on_error_async(context).await
    }

    fn is_end(&self, context: &CmpContext) -> bool {
        self.inner.is_end(context)
    }

    async fn is_end_async(&self, context: &CmpContext) -> Result<bool, LiteflowError> {
        self.inner.is_end_async(context).await
    }

    fn is_rollback(&self) -> bool {
        self.inner.is_rollback()
    }

    async fn rollback(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        self.inner.rollback(ctx).await
    }

    async fn do_rollback(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        // 与 execute 相同，保留内部代理/声明式组件覆盖 doRollback 的能力。
        self.inner.do_rollback(ctx).await
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn display_name_async(
        &self,
        context: &CmpContext,
    ) -> Result<Option<String>, LiteflowError> {
        self.inner.display_name_async(context).await
    }

    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn node_type(&self) -> Option<NodeTypeEnum> {
        Some(self.node_type)
    }

    fn has_explicit_node_type(&self) -> bool {
        self.node_type_explicit
    }

    fn retry_count(&self) -> usize {
        let component_retry_count = self.inner.retry_count();
        if component_retry_count == 0 {
            self.default_retry_count.unwrap_or_else(|| {
                // Java ComponentInitializer 每次初始化都读取 LiteflowConfigGetter。
                // Rust 允许先注册组件再创建 FlowExecutor，因此延迟到执行期读取，
                // 仍能让随后装配的全局配置进入真实 NodeExecutor 重试循环。
                #[allow(deprecated)]
                let retry_count = LiteflowConfigGetter::get().get_retry_count();
                retry_count as usize
            })
        } else {
            component_retry_count
        }
    }

    fn is_retry_for(&self, error: &LiteflowError) -> bool {
        self.inner.is_retry_for(error)
    }

    fn node_executor(&self) -> Option<Arc<dyn NodeExecutor>> {
        self.inner
            .node_executor()
            .or_else(|| self.default_node_executor.clone())
    }

    async fn node_executor_class_async(
        &self,
        context: &CmpContext,
    ) -> Result<Option<String>, LiteflowError> {
        self.inner.node_executor_class_async(context).await
    }

    fn unload_script(&self, node_id: &str) -> Result<bool, LiteflowError> {
        self.inner.unload_script(node_id)
    }
}
