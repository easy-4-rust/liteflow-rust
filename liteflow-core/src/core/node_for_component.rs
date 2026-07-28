//! FOR 计数节点组件。

use std::future::Future;

use async_trait::async_trait;
use serde_json::{Number, Value};

use crate::core::NodeComponent;
use crate::exception::LiteflowError;
use crate::slot::CmpContext;

/// 把返回循环次数的异步处理器适配为 `NodeComponent`。
///
/// 对应 Java: `com.yomahub.liteflow.core.NodeForComponent`。
pub struct NodeForComponent<F> {
    name: String,
    process_for: F,
}

impl<F> NodeForComponent<F> {
    /// 使用组件名与计数处理器创建节点。
    #[must_use]
    pub fn new(name: impl Into<String>, process_for: F) -> Self {
        Self {
            name: name.into(),
            process_for,
        }
    }

    /// 执行 FOR 节点核心逻辑并返回非负循环次数。
    ///
    /// 对应 Java: `NodeForComponent#processFor`。
    pub async fn process_for<Fut>(&self, ctx: CmpContext) -> Result<usize, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<usize, LiteflowError>>,
    {
        (self.process_for)(ctx).await
    }

    /// 执行 FOR 组件并转换为统一节点结果。
    ///
    /// 参数 `ctx` 为当前节点执行上下文；返回非负循环次数对应的 JSON 数字。
    /// 对应 Java: `NodeForComponent#process`。
    pub async fn process<Fut>(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<usize, LiteflowError>>,
    {
        let count = self.process_for(ctx.clone()).await?;
        let count = u64::try_from(count)
            .map_err(|_| LiteflowError::NodeTypeNotSupport("FOR count exceeds u64".to_string()))?;
        Ok(Value::Number(Number::from(count)))
    }

    /// 返回当前节点最近一次成功执行的循环次数。
    ///
    /// 参数 `ctx` 提供任务 Frame 与节点实例标识；尚未执行时返回 `None`。
    /// 对应 Java: `NodeForComponent#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.frame.get_node_item_result(ctx.node.display())
    }
}

#[async_trait]
impl<F, Fut> NodeComponent for NodeForComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<usize, LiteflowError>> + Send,
{
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        NodeForComponent::process(self, ctx).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}
