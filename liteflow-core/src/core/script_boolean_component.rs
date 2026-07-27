//! 对应 Java: `com.yomahub.liteflow.core.ScriptBooleanComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::{NodeComponent, ScriptComponent};
use crate::script::{ScriptExecutor, ScriptKind};
use crate::{CmpContext, LFResult, LiteflowError};

/// 布尔脚本节点，用于 IF/WHILE/BREAK 条件判断。
pub struct ScriptBooleanComponent {
    script_component: ScriptComponent,
}

impl ScriptBooleanComponent {
    /// 编译脚本并创建布尔脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }

    /// 执行布尔脚本并返回条件结果。
    ///
    /// 参数 `context` 对应 Java 当前组件执行上下文。对应 Java:
    /// `ScriptBooleanComponent#processBoolean`。
    pub async fn process_boolean(&self, context: &CmpContext) -> LFResult<bool> {
        let value = self.script_component.process_script(context)?;
        let value = ScriptKind::Boolean.check_return(self.name(), value)?;
        Ok(value
            .as_bool()
            .expect("ScriptKind::Boolean 已验证返回值为 bool"))
    }

    /// 重新加载当前节点脚本。对应 Java:
    /// `ScriptBooleanComponent#loadScript`。
    pub fn load_script(&self, script: &str, language: &str) -> LFResult<()> {
        self.script_component.load_script(script, language)
    }

    /// 判断当前脚本节点是否允许执行。对应 Java:
    /// `ScriptBooleanComponent#isAccess`。
    #[must_use]
    pub fn is_access(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_access(&wrap, context)
    }

    /// 判断脚本错误后是否继续。对应 Java:
    /// `ScriptBooleanComponent#isContinueOnError`。
    #[must_use]
    pub fn is_continue_on_error(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_continue_on_error(&wrap, context)
    }

    /// 返回流程是否已经结束。对应 Java: `ScriptBooleanComponent#isEnd`。
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
    /// `ScriptBooleanComponent#beforeProcess`。
    pub async fn before_process(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_before_process(&wrap, context);
        Ok(())
    }

    /// 执行脚本 finally 后置切面。对应 Java:
    /// `ScriptBooleanComponent#afterProcess`。
    pub async fn after_process(&self, context: &CmpContext) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_after_process(&wrap, context);
    }

    /// 执行脚本成功切面。对应 Java: `ScriptBooleanComponent#onSuccess`。
    pub async fn on_success(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_success(&wrap, context);
        Ok(())
    }

    /// 执行脚本失败切面。对应 Java: `ScriptBooleanComponent#onError`。
    pub async fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_error(&wrap, context, error);
    }

    /// 回滚脚本组件。对应 Java: `ScriptBooleanComponent#rollback`。
    pub async fn rollback(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_rollback(&wrap, context)
    }
}

#[async_trait]
impl NodeComponent for ScriptBooleanComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        self.process_boolean(ctx).await.map(Value::Bool)
    }

    async fn before_process(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptBooleanComponent::before_process(self, ctx).await
    }

    async fn on_success(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptBooleanComponent::on_success(self, ctx).await
    }

    async fn after_process(&self, ctx: &CmpContext) {
        ScriptBooleanComponent::after_process(self, ctx).await;
    }

    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        ScriptBooleanComponent::on_error(self, ctx, error).await;
    }

    fn is_access(&self, ctx: &CmpContext) -> bool {
        ScriptBooleanComponent::is_access(self, ctx)
    }

    fn is_continue_on_error_with_context(&self, ctx: &CmpContext) -> bool {
        ScriptBooleanComponent::is_continue_on_error(self, ctx)
    }

    fn is_end(&self, ctx: &CmpContext) -> bool {
        ScriptBooleanComponent::is_end(self, ctx)
    }

    fn is_rollback(&self) -> bool {
        true
    }

    async fn rollback(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptBooleanComponent::rollback(self, ctx).await
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }

    fn unload_script(&self, _node_id: &str) -> LFResult<bool> {
        self.script_component.unload()?;
        Ok(true)
    }
}
