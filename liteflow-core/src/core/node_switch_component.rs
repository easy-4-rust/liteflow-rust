//! SWITCH 路由节点组件。

use std::future::Future;

use async_trait::async_trait;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::exception::LiteflowError;
use crate::slot::CmpContext;

/// 把返回目标节点 id 的异步处理器适配为 `NodeComponent`。
///
/// 返回值支持 Java LiteFlow 的 `node_id` 与 `node_id:tag` 两种形式，具体目标
/// 校验仍由 `SwitchCondition` 承担。
///
/// 对应 Java: `com.yomahub.liteflow.core.NodeSwitchComponent`。
pub struct NodeSwitchComponent<F> {
    name: String,
    process_switch: F,
}

impl<F> NodeSwitchComponent<F> {
    /// 使用组件名与路由处理器创建节点。
    #[must_use]
    pub fn new(name: impl Into<String>, process_switch: F) -> Self {
        Self {
            name: name.into(),
            process_switch,
        }
    }

    /// 执行 SWITCH 节点核心逻辑并返回目标节点 id。
    ///
    /// 对应 Java: `NodeSwitchComponent#processSwitch`。
    pub async fn process_switch<Fut>(&self, ctx: CmpContext) -> Result<String, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<String, LiteflowError>>,
    {
        (self.process_switch)(ctx).await
    }
}

#[async_trait]
impl<F, Fut> NodeComponent for NodeSwitchComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<String, LiteflowError>> + Send,
{
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.process_switch(ctx.clone()).await.map(Value::String)
    }

    fn name(&self) -> &str {
        &self.name
    }
}
