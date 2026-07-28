//! BOOLEAN 类型节点组件。

use std::future::Future;

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::exception::LiteflowError;
use crate::slot::CmpContext;

/// 把返回布尔值的异步处理器适配为 `NodeComponent`。
///
/// `process` 会把 `process_boolean` 的结果写成 `Value::Bool`，由 IF、WHILE、
/// BREAK、AND/OR/NOT 条件按现有 Node 执行链消费。
///
/// 对应 Java: `com.yomahub.liteflow.core.NodeBooleanComponent`。
pub struct NodeBooleanComponent<F> {
    name: String,
    process_boolean: F,
}

impl<F> NodeBooleanComponent<F> {
    /// 使用组件名与布尔处理器创建节点。
    #[must_use]
    pub fn new(name: impl Into<String>, process_boolean: F) -> Self {
        Self {
            name: name.into(),
            process_boolean,
        }
    }

    /// 执行布尔节点的核心逻辑。
    ///
    /// 参数 `ctx` 对应 Java 当前 Slot/RefNode 上下文；返回条件布尔值。
    /// 对应 Java: `NodeBooleanComponent#processBoolean`。
    pub async fn process_boolean<Fut>(&self, ctx: CmpContext) -> Result<bool, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<bool, LiteflowError>>,
    {
        (self.process_boolean)(ctx).await
    }

    /// 执行 BOOLEAN 组件并转换为统一节点结果。
    ///
    /// 参数 `ctx` 为当前节点执行上下文；返回 `Value::Bool`，供 IF、WHILE、
    /// BREAK 与布尔组合条件消费。对应 Java: `NodeBooleanComponent#process`。
    pub async fn process<Fut>(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<bool, LiteflowError>>,
    {
        self.process_boolean(ctx.clone()).await.map(Value::Bool)
    }

    /// 返回当前节点最近一次成功执行的布尔结果。
    ///
    /// 参数 `ctx` 提供任务 Frame 与节点实例标识；尚未执行时返回 `None`。
    /// 对应 Java: `NodeBooleanComponent#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.frame.get_node_item_result(ctx.node.display())
    }
}

#[async_trait]
impl<F, Fut> NodeComponent for NodeBooleanComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<bool, LiteflowError>> + Send,
{
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        NodeBooleanComponent::process(self, ctx).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}
