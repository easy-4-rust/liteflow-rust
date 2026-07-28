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

    /// 执行 SWITCH 组件并转换为统一节点结果。
    ///
    /// 参数 `ctx` 为当前节点执行上下文；返回 `Value::String` 形式的目标节点 id
    /// 或 `node_id:tag`。对应 Java: `NodeSwitchComponent#process`。
    pub async fn process<Fut>(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>
    where
        F: Fn(CmpContext) -> Fut,
        Fut: Future<Output = Result<String, LiteflowError>>,
    {
        self.process_switch(ctx.clone()).await.map(Value::String)
    }

    /// 返回当前节点最近一次成功执行的路由目标。
    ///
    /// 参数 `ctx` 提供任务 Frame 与节点实例标识；尚未执行时返回 `None`。
    /// 对应 Java: `NodeSwitchComponent#getItemResultMetaValue`。
    #[must_use]
    pub fn get_item_result_meta_value(&self, ctx: &CmpContext) -> Option<Value> {
        ctx.frame.get_node_item_result(ctx.node.display())
    }

    /// 返回当前 SWITCH 表达式允许跳转的目标节点 ID。
    ///
    /// 参数 `ctx` 是当前路由节点的组件上下文；返回值对应 Java
    /// `NodeSwitchComponent#getTargetList`。若组件不在 SWITCH 条件中执行，
    /// 返回空列表。
    #[must_use]
    pub fn get_target_list(&self, ctx: &CmpContext) -> Vec<String> {
        ctx.switch_target_list()
    }
}

#[async_trait]
impl<F, Fut> NodeComponent for NodeSwitchComponent<F>
where
    F: Fn(CmpContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<String, LiteflowError>> + Send,
{
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        NodeSwitchComponent::process(self, ctx).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}
