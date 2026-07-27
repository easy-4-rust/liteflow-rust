//! 对应 Java: `com.yomahub.liteflow.core.ScriptForComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::{NodeComponent, ScriptComponent};
use crate::script::{ScriptExecutor, ScriptKind};
use crate::{CmpContext, LFResult, LiteflowError};

/// 循环次数脚本节点，返回非负数值。
pub struct ScriptForComponent {
    script_component: ScriptComponent,
}

impl ScriptForComponent {
    /// 编译脚本并创建循环次数脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }

    /// 执行循环次数脚本并返回非负次数。
    ///
    /// 参数 `context` 对应 Java 当前组件执行上下文。对应 Java:
    /// `ScriptForComponent#processFor`。
    pub async fn process_for(&self, context: &CmpContext) -> LFResult<usize> {
        let value = self.script_component.process_script(context)?;
        let value = ScriptKind::For.check_return(self.name(), value)?;
        let count = value.as_u64().ok_or_else(|| LiteflowError::NodeTypeError {
            node: self.name().to_string(),
            expect: "non-negative integer".to_string(),
            actual: value.to_string(),
        })?;
        usize::try_from(count).map_err(|_| LiteflowError::NodeTypeError {
            node: self.name().to_string(),
            expect: "usize range".to_string(),
            actual: count.to_string(),
        })
    }

    /// 重新加载当前节点脚本。对应 Java: `ScriptForComponent#loadScript`。
    pub fn load_script(&self, script: &str, language: &str) -> LFResult<()> {
        self.script_component.load_script(script, language)
    }

    /// 判断当前脚本节点是否允许执行。对应 Java:
    /// `ScriptForComponent#isAccess`。
    #[must_use]
    pub fn is_access(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_access(&wrap, context)
    }

    /// 判断脚本错误后是否继续。对应 Java:
    /// `ScriptForComponent#isContinueOnError`。
    #[must_use]
    pub fn is_continue_on_error(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_continue_on_error(&wrap, context)
    }

    /// 返回流程是否已经结束。对应 Java: `ScriptForComponent#isEnd`。
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

    /// 执行脚本前置切面。对应 Java: `ScriptForComponent#beforeProcess`。
    pub async fn before_process(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_before_process(&wrap, context);
        Ok(())
    }

    /// 执行脚本 finally 后置切面。对应 Java:
    /// `ScriptForComponent#afterProcess`。
    pub async fn after_process(&self, context: &CmpContext) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_after_process(&wrap, context);
    }

    /// 执行脚本成功切面。对应 Java: `ScriptForComponent#onSuccess`。
    pub async fn on_success(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_success(&wrap, context);
        Ok(())
    }

    /// 执行脚本失败切面。对应 Java: `ScriptForComponent#onError`。
    pub async fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_error(&wrap, context, error);
    }

    /// 回滚脚本组件。对应 Java: `ScriptForComponent#rollback`。
    pub async fn rollback(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_rollback(&wrap, context)
    }
}

#[async_trait]
impl NodeComponent for ScriptForComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        let count = self.process_for(ctx).await?;
        Ok(Value::Number(serde_json::Number::from(count as u64)))
    }

    async fn before_process(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptForComponent::before_process(self, ctx).await
    }

    async fn on_success(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptForComponent::on_success(self, ctx).await
    }

    async fn after_process(&self, ctx: &CmpContext) {
        ScriptForComponent::after_process(self, ctx).await;
    }

    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        ScriptForComponent::on_error(self, ctx, error).await;
    }

    fn is_access(&self, ctx: &CmpContext) -> bool {
        ScriptForComponent::is_access(self, ctx)
    }

    fn is_continue_on_error_with_context(&self, ctx: &CmpContext) -> bool {
        ScriptForComponent::is_continue_on_error(self, ctx)
    }

    fn is_end(&self, ctx: &CmpContext) -> bool {
        ScriptForComponent::is_end(self, ctx)
    }

    fn is_rollback(&self) -> bool {
        true
    }

    async fn rollback(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptForComponent::rollback(self, ctx).await
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }

    fn unload_script(&self, _node_id: &str) -> LFResult<bool> {
        self.script_component.unload()?;
        Ok(true)
    }
}
