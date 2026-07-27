//! 对应 Java: `com.yomahub.liteflow.core.ScriptSwitchComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::{NodeComponent, ScriptComponent};
use crate::script::{ScriptExecutor, ScriptKind};
use crate::{CmpContext, LFResult, LiteflowError};

/// 选择脚本节点，返回目标节点或标签字符串。
pub struct ScriptSwitchComponent {
    script_component: ScriptComponent,
}

impl ScriptSwitchComponent {
    /// 编译脚本并创建选择脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }

    /// 执行选择脚本并返回目标 ID 或标签。
    ///
    /// Java 返回值可为 null，Rust 使用 `Option<String>` 表达。对应 Java:
    /// `ScriptSwitchComponent#processSwitch`。
    pub async fn process_switch(&self, context: &CmpContext) -> LFResult<Option<String>> {
        let value = self.script_component.process_script(context)?;
        let value = ScriptKind::Switch.check_return(self.name(), value)?;
        Ok(value.as_str().map(str::to_string))
    }

    /// 重新加载当前节点脚本。对应 Java:
    /// `ScriptSwitchComponent#loadScript`。
    pub fn load_script(&self, script: &str, language: &str) -> LFResult<()> {
        self.script_component.load_script(script, language)
    }

    /// 判断当前脚本节点是否允许执行。对应 Java:
    /// `ScriptSwitchComponent#isAccess`。
    #[must_use]
    pub fn is_access(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_access(&wrap, context)
    }

    /// 判断脚本错误后是否继续。对应 Java:
    /// `ScriptSwitchComponent#isContinueOnError`。
    #[must_use]
    pub fn is_continue_on_error(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_continue_on_error(&wrap, context)
    }

    /// 返回流程是否已经结束。对应 Java: `ScriptSwitchComponent#isEnd`。
    #[must_use]
    pub fn is_end(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        context
            .inner
            .ended
            .load(std::sync::atomic::Ordering::Acquire)
            || self
                .script_component
                .executor()
                .execute_is_end(&wrap, context)
    }

    /// 执行脚本前置切面。对应 Java:
    /// `ScriptSwitchComponent#beforeProcess`。
    pub async fn before_process(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_before_process(&wrap, context);
        Ok(())
    }

    /// 执行脚本 finally 后置切面。对应 Java:
    /// `ScriptSwitchComponent#afterProcess`。
    pub async fn after_process(&self, context: &CmpContext) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_after_process(&wrap, context);
    }

    /// 执行脚本成功切面。对应 Java: `ScriptSwitchComponent#onSuccess`。
    pub async fn on_success(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_success(&wrap, context);
        Ok(())
    }

    /// 执行脚本失败切面。对应 Java: `ScriptSwitchComponent#onError`。
    pub async fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_error(&wrap, context, error);
    }

    /// 回滚脚本组件。对应 Java: `ScriptSwitchComponent#rollback`。
    pub async fn rollback(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_rollback(&wrap, context)
    }
}

#[async_trait]
impl NodeComponent for ScriptSwitchComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        Ok(self
            .process_switch(ctx)
            .await?
            .map(Value::String)
            .unwrap_or(Value::Null))
    }

    async fn before_process(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptSwitchComponent::before_process(self, ctx).await
    }

    async fn on_success(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptSwitchComponent::on_success(self, ctx).await
    }

    async fn after_process(&self, ctx: &CmpContext) {
        ScriptSwitchComponent::after_process(self, ctx).await;
    }

    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        ScriptSwitchComponent::on_error(self, ctx, error).await;
    }

    fn is_access(&self, ctx: &CmpContext) -> bool {
        ScriptSwitchComponent::is_access(self, ctx)
    }

    fn is_continue_on_error_with_context(&self, ctx: &CmpContext) -> bool {
        ScriptSwitchComponent::is_continue_on_error(self, ctx)
    }

    fn is_end(&self, ctx: &CmpContext) -> bool {
        ScriptSwitchComponent::is_end(self, ctx)
    }

    fn is_rollback(&self) -> bool {
        true
    }

    async fn rollback(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptSwitchComponent::rollback(self, ctx).await
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }

    fn unload_script(&self, _node_id: &str) -> LFResult<bool> {
        self.script_component.unload()?;
        Ok(true)
    }
}
