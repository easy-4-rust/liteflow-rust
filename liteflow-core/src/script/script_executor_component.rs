//! 通用脚本执行器到 LiteFlow 节点组件的适配对象。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::{ScriptExecutor, ScriptKind};
use crate::core::NodeComponent;
use crate::exception::{LFResult, LiteflowError};
use crate::script::ScriptExecuteWrap;
use crate::slot::CmpContext;

/// 将一个已经加载脚本的 `ScriptExecutor` 适配为对应节点类别。
///
/// Java 的 `ScriptCommonComponent`、`ScriptBooleanComponent`、
/// `ScriptSwitchComponent`、`ScriptForComponent` 和 `ScriptIteratorComponent`
/// 都把执行委托给 `ScriptExecutorFactory`。Rust 用本对象保存强类型执行器句柄，
/// 避免每个语言插件再次复制五种组件壳。对应 Java:
/// `com.yomahub.liteflow.core.ScriptComponent` 及其子类。
pub struct ScriptExecutorComponent {
    node_id: String,
    kind: ScriptKind,
    executor: Arc<dyn ScriptExecutor>,
}

impl ScriptExecutorComponent {
    /// 创建脚本节点适配器。
    ///
    /// `node_id` 必须已经通过 `executor.load` 加载，`kind` 决定返回值约束。
    pub fn new(
        node_id: impl Into<String>,
        kind: ScriptKind,
        executor: Arc<dyn ScriptExecutor>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            kind,
            executor,
        }
    }

    /// 返回底层脚本执行器。
    #[must_use]
    pub fn executor(&self) -> &Arc<dyn ScriptExecutor> {
        &self.executor
    }
}

#[async_trait]
impl NodeComponent for ScriptExecutorComponent {
    /// 执行缓存脚本并按节点种类校验返回值。
    ///
    /// 对应 Java 各 `Script*Component#process*` 方法。
    async fn process(&self, context: &CmpContext) -> LFResult<Value> {
        let value = self.executor.execute(&self.node_id, context)?;
        self.kind.check_return(&self.node_id, value)
    }

    async fn before_process(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = ScriptExecuteWrap::from_context(context);
        self.executor.execute_before_process(&wrap, context);
        Ok(())
    }

    async fn on_success(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = ScriptExecuteWrap::from_context(context);
        self.executor.execute_on_success(&wrap, context);
        Ok(())
    }

    async fn after_process(&self, context: &CmpContext) {
        let wrap = ScriptExecuteWrap::from_context(context);
        self.executor.execute_after_process(&wrap, context);
    }

    async fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        let wrap = ScriptExecuteWrap::from_context(context);
        self.executor.execute_on_error(&wrap, context, error);
    }

    fn is_access(&self, context: &CmpContext) -> bool {
        let wrap = ScriptExecuteWrap::from_context(context);
        self.executor.execute_is_access(&wrap, context)
    }

    fn is_continue_on_error(&self) -> bool {
        false
    }

    fn is_continue_on_error_with_context(&self, context: &CmpContext) -> bool {
        let wrap = ScriptExecuteWrap::from_context(context);
        self.executor.execute_is_continue_on_error(&wrap, context)
    }

    fn is_end(&self, context: &CmpContext) -> bool {
        let wrap = ScriptExecuteWrap::from_context(context);
        context
            .inner
            .ended
            .load(std::sync::atomic::Ordering::Acquire)
            || self.executor.execute_is_end(&wrap, context)
    }

    fn is_rollback(&self) -> bool {
        true
    }

    async fn rollback(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = ScriptExecuteWrap::from_context(context);
        self.executor.execute_rollback(&wrap, context)
    }

    fn name(&self) -> &str {
        &self.node_id
    }

    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn unload_script(&self, node_id: &str) -> LFResult<bool> {
        self.executor.unload(node_id)?;
        Ok(true)
    }
}
