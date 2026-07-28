//! ITERATOR 迭代节点组件。

use std::future::Future;

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::exception::LiteflowError;
use crate::slot::CmpContext;

/// 把返回 JSON 值序列的异步处理器适配为 `NodeComponent`。
///
/// Java 返回 `Iterator<?>`；Rust 以拥有所有权的 `Vec<Value>` 表达可安全跨
/// async 边界的同等迭代数据，运行时以 `Value::Array` 传给 ITERATOR 条件。
///
/// 对应 Java: `com.yomahub.liteflow.core.NodeIteratorComponent`。
pub struct NodeIteratorComponent<F> {
    name: String,
    process_iterator: F,
}

impl<F> NodeIteratorComponent<F> {
    /// 使用组件名与迭代数据处理器创建节点。
    #[must_use]
    pub fn new(name: impl Into<String>, process_iterator: F) -> Self {
        Self {
            name: name.into(),
            process_iterator,
        }
    }

    /// 执行 ITERATOR 节点核心逻辑并返回迭代数据。
    ///
    /// 对应 Java: `NodeIteratorComponent#processIterator`。
    pub async fn process_iterator<Fut>(&self, ctx: CmpContext) -> Result<Vec<Value>, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<Vec<Value>, LiteflowError>>,
    {
        (self.process_iterator)(ctx).await
    }

    /// 执行 ITERATOR 组件并转换为统一节点结果。
    ///
    /// 参数 `ctx` 为当前节点执行上下文；返回 `Value::Array`，由迭代条件逐项消费。
    /// 对应 Java: `NodeIteratorComponent#process`。
    pub async fn process<Fut>(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<Vec<Value>, LiteflowError>>,
    {
        self.process_iterator(ctx.clone()).await.map(Value::Array)
    }

    /// 返回当前节点最近一次成功执行的迭代数据。
    ///
    /// 参数 `ctx` 提供任务 Frame 与节点实例标识；尚未执行时返回 `None`。
    /// 对应 Java: `NodeIteratorComponent#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.frame.get_node_item_result(ctx.node.display())
    }
}

#[async_trait]
impl<F, Fut> NodeComponent for NodeIteratorComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<Value>, LiteflowError>> + Send,
{
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        NodeIteratorComponent::process(self, ctx).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}
