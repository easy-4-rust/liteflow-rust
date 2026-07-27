//! 对应 Java: `com.yomahub.liteflow.core.ScriptCommonComponent`。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::{NodeComponent, ScriptComponent};
use crate::script::ScriptExecutor;
use crate::{CmpContext, LFResult, LiteflowError};

/// 普通脚本节点，直接返回脚本执行结果。
pub struct ScriptCommonComponent {
    script_component: ScriptComponent,
}

impl ScriptCommonComponent {
    /// 编译脚本并创建普通脚本组件。
    pub fn new(node_id: &str, script: &str) -> LFResult<Self> {
        Ok(Self {
            script_component: ScriptComponent::new(node_id, script)?,
        })
    }

    /// 执行普通脚本并返回脚本结果。
    ///
    /// 参数 `context` 对应 Java 组件 ThreadLocal 持有的当前 Slot/RefNode。
    /// 对应 Java: `ScriptCommonComponent#process`。
    pub async fn process(&self, context: &CmpContext) -> LFResult<Value> {
        self.script_component.process_script(context)
    }

    /// 重新加载当前节点脚本。
    ///
    /// 参数 `script`、`language` 对应 Java 同名参数。对应 Java:
    /// `ScriptCommonComponent#loadScript`。
    pub fn load_script(&self, script: &str, language: &str) -> LFResult<()> {
        self.script_component.load_script(script, language)
    }

    /// 判断当前脚本节点是否允许执行。对应 Java:
    /// `ScriptCommonComponent#isAccess`。
    #[must_use]
    pub fn is_access(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_access(&wrap, context)
    }

    /// 判断脚本错误后是否继续。对应 Java:
    /// `ScriptCommonComponent#isContinueOnError`。
    #[must_use]
    pub fn is_continue_on_error(&self, context: &CmpContext) -> bool {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_is_continue_on_error(&wrap, context)
    }

    /// 返回流程是否已经结束。
    ///
    /// Rhai 对应 Java 非 `java` 语言分支，先读取通用 Slot 标记，同时允许执行器
    /// 扩展结束判断。对应 Java: `ScriptCommonComponent#isEnd`。
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
    /// `ScriptCommonComponent#beforeProcess`。
    pub async fn before_process(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_before_process(&wrap, context);
        Ok(())
    }

    /// 执行脚本 finally 后置切面。对应 Java:
    /// `ScriptCommonComponent#afterProcess`。
    pub async fn after_process(&self, context: &CmpContext) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_after_process(&wrap, context);
    }

    /// 执行脚本成功切面。对应 Java: `ScriptCommonComponent#onSuccess`。
    pub async fn on_success(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_success(&wrap, context);
        Ok(())
    }

    /// 执行脚本失败切面。
    ///
    /// 参数 `error` 为原始执行错误。对应 Java:
    /// `ScriptCommonComponent#onError`。
    pub async fn on_error(&self, context: &CmpContext, error: &LiteflowError) {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_on_error(&wrap, context, error);
    }

    /// 回滚脚本组件。对应 Java: `ScriptCommonComponent#rollback`。
    pub async fn rollback(&self, context: &CmpContext) -> LFResult<()> {
        let wrap = self.script_component.build_wrap(context);
        self.script_component
            .executor()
            .execute_rollback(&wrap, context)
    }
}

#[async_trait]
impl NodeComponent for ScriptCommonComponent {
    async fn process(&self, ctx: &CmpContext) -> LFResult<Value> {
        ScriptCommonComponent::process(self, ctx).await
    }

    async fn before_process(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptCommonComponent::before_process(self, ctx).await
    }

    async fn on_success(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptCommonComponent::on_success(self, ctx).await
    }

    async fn after_process(&self, ctx: &CmpContext) {
        ScriptCommonComponent::after_process(self, ctx).await;
    }

    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        ScriptCommonComponent::on_error(self, ctx, error).await;
    }

    fn is_access(&self, ctx: &CmpContext) -> bool {
        ScriptCommonComponent::is_access(self, ctx)
    }

    fn is_continue_on_error_with_context(&self, ctx: &CmpContext) -> bool {
        ScriptCommonComponent::is_continue_on_error(self, ctx)
    }

    fn is_end(&self, ctx: &CmpContext) -> bool {
        ScriptCommonComponent::is_end(self, ctx)
    }

    fn is_rollback(&self) -> bool {
        true
    }

    async fn rollback(&self, ctx: &CmpContext) -> LFResult<()> {
        ScriptCommonComponent::rollback(self, ctx).await
    }

    fn name(&self) -> &str {
        self.script_component.node_id()
    }

    fn unload_script(&self, _node_id: &str) -> LFResult<bool> {
        self.script_component.unload()?;
        Ok(true)
    }
}
