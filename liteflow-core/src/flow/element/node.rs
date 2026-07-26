//! 对应 flow.element.Node：包装组件实例的可执行节点。
//! execute_once() 对齐 Java Node.execute → processFlow 单次执行语义：
//! isAccess → beforeProcess → process → afterProcess，
//! 异常时 onError → isContinueOnError 决定是否吞掉，全部记入 CmpStep。
//! Executable::execute 则对齐 Java Node.execute(slotIndex) 的完整入口：
//! 经 NodeExecutorHelper 取得节点执行器（对应 NodeExecutor.execute(instance)），
//! 由执行器的重试主干循环调用 execute_once。

use crate::core::node_component::NodeComponent;
use crate::el::NodeRef;
use crate::enums::CmpStepTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::element::rollbackable::Rollbackable;
use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::{CmpContext, Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use super::NodeHooks;

pub struct Node {
    node_ref: NodeRef,
    instance: Arc<dyn NodeComponent>,
    /// 实例编号（NodeInstanceIdManageSpi；同节点多次出现时编号）
    node_instance_id: Option<String>,
    hooks: NodeHooks,
}

impl Node {
    pub fn new(node_ref: NodeRef, instance: Arc<dyn NodeComponent>) -> Self {
        Self {
            node_ref,
            instance,
            node_instance_id: None,
            hooks: NodeHooks::default(),
        }
    }

    pub fn with_instance_id(mut self, instance_id: impl Into<String>) -> Self {
        self.node_instance_id = Some(instance_id.into());
        self
    }

    pub fn with_hooks(mut self, hooks: NodeHooks) -> Self {
        self.hooks = hooks;
        self
    }

    pub fn node_instance_id(&self) -> Option<&str> {
        self.node_instance_id.as_deref()
    }

    /// 设置节点实例编号。对应 Java `Node#setNodeInstanceId`。
    pub fn set_node_instance_id(&mut self, instance_id: impl Into<String>) {
        self.node_instance_id = Some(instance_id.into());
    }

    pub fn node_ref(&self) -> &NodeRef {
        &self.node_ref
    }

    pub fn instance(&self) -> &Arc<dyn NodeComponent> {
        &self.instance
    }

    /// getDisplayName()（优先别名）
    pub fn display_name(&self) -> &str {
        self.node_ref.display()
    }

    /// 单次执行逻辑（对应 Java NodeComponent.execute() 被 NodeExecutor 重试循环
    /// 反复调用的那一次执行）：isAccess → beforeProcess → process → afterProcess，
    /// 异常时 onError → isContinueOnError 决定是否吞掉，全部记入 CmpStep。
    /// 重试语义由 flow.executor.NodeExecutor 承担，本方法不含重试。
    pub async fn execute_once(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        if ctx.is_ended() {
            return Err(LiteflowError::ChainEnd);
        }
        let cctx = CmpContext {
            inner: ctx.inner.clone(),
            node: self.node_ref.clone(),
            frame: frame.clone(),
        };

        // isAccess
        if !self.instance.is_access(&cctx) {
            return Ok(Value::Null);
        }

        let mut step = CmpStep::new(self.display_name().to_string(), CmpStepTypeEnum::Single);
        step.tag = self.node_ref.tag.clone();
        step.node_name = self.instance.name().to_string();

        // Java 在 NodeComponent.execute() 开始时就把 instance/refNode 写入 CmpStep。
        // Rust 端显式登记内部回滚目标；重试会重复登记，但真正回滚时按
        // NodeInstanceId 去重，对齐 NodeComponent#doRollback。
        if self.instance.is_rollback() {
            let node_instance_id = self
                .node_instance_id
                .clone()
                .unwrap_or_else(|| self.node_ref.display().to_string());
            ctx.register_rollback(node_instance_id, self.instance.clone(), cctx.clone());
        }

        // 全局切面 beforeProcess（对应 aop.ICmpAroundAspect）
        for aspect in &self.hooks.aspects {
            aspect.before_process(&cctx).await;
        }

        // 对齐 Java NodeComponent#execute：
        // beforeProcess → process → onSuccess；任一步骤失败都进入 onError；
        // afterProcess 始终在 finally 语义中执行。
        let result = match self.instance.before_process(&cctx).await {
            Ok(()) => match self.instance.process(&cctx).await {
                Ok(value) => self.instance.on_success(&cctx).await.map(|_| value),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };

        match &result {
            Ok(_) => {
                for aspect in &self.hooks.aspects {
                    aspect.on_success(&cctx).await;
                }
            }
            Err(error) => {
                self.instance.on_error(&cctx, error).await;
                for aspect in &self.hooks.aspects {
                    aspect.on_error(&cctx, error).await;
                }
                if !matches!(error, LiteflowError::ChainEnd) {
                    ctx.set_exception(&error.to_string());
                }
            }
        }

        self.instance.after_process(&cctx).await;
        for aspect in &self.hooks.aspects {
            aspect.after_process(&cctx).await;
        }

        match result {
            Ok(v) => {
                step.finish(true, None);
                if let Some(m) = &self.hooks.monitor {
                    m.record(
                        self.display_name(),
                        step.time_spent.unwrap_or_default(),
                        true,
                    );
                }
                ctx.record_step(step);
                // setIsEnd(true) 语义
                if ctx.is_ended() {
                    return Err(LiteflowError::ChainEnd);
                }
                Ok(v)
            }
            Err(LiteflowError::ChainEnd) => {
                step.finish(false, Some(LiteflowError::ChainEnd.to_string()));
                if let Some(m) = &self.hooks.monitor {
                    m.record(
                        self.display_name(),
                        step.time_spent.unwrap_or_default(),
                        false,
                    );
                }
                ctx.record_step(step);
                Err(LiteflowError::ChainEnd)
            }
            Err(e) => {
                let error_kind = format!("{e:?}")
                    .split([' ', '(', '{'])
                    .next()
                    .unwrap_or_default()
                    .to_string();
                step.finish(false, Some(e.to_string()));
                if let Some(m) = &self.hooks.monitor {
                    m.record(
                        self.display_name(),
                        step.time_spent.unwrap_or_default(),
                        false,
                    );
                }
                ctx.record_step(step);
                if self.instance.is_continue_on_error() {
                    return Ok(Value::Null);
                }
                Err(LiteflowError::NodeExec {
                    node: self.display_name().to_string(),
                    msg: e.to_string(),
                    kind: error_kind,
                })
            }
        }
    }
}

#[async_trait]
impl Executable for Node {
    /// 对应 Java Node.execute(slotIndex)：经 NodeExecutorHelper.buildNodeExecutor
    /// 取得节点执行器（组件未指定时为缓存的 DefaultNodeExecutor），
    /// 委托执行器的重试主干执行（NodeExecutor.execute → 循环调用 execute_once）。
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let executor = crate::flow::executor::NodeExecutorHelper::load_instance()
            .build_node_executor(self.instance.node_executor());
        executor.execute(self, ctx, frame).await
    }

    fn execute_type(&self) -> crate::enums::ExecuteableTypeEnum {
        crate::enums::ExecuteableTypeEnum::Node
    }

    fn id(&self) -> &str {
        self.node_ref.display()
    }

    fn tag(&self) -> Option<&str> {
        self.node_ref.tag.as_deref()
    }

    /// isAccess(slotIndex)（2.16：AND/OR 求值前的过滤依据）
    async fn is_access(&self, ctx: &Ctx, frame: &Frame) -> bool {
        let cctx = CmpContext {
            inner: ctx.inner.clone(),
            node: self.node_ref.clone(),
            frame: frame.clone(),
        };
        self.instance.is_access(&cctx)
    }
}

#[async_trait]
impl Rollbackable for Node {
    /// 调用组件补偿逻辑并记录 rollback step。
    ///
    /// 与 Java `Node#rollback` 一致，组件回滚错误只记录为失败步骤，不覆盖触发
    /// 补偿的原始流程错误。
    async fn rollback(&self, ctx: &Ctx, frame: &Frame) -> LFResult<()> {
        let component_context = CmpContext {
            inner: ctx.inner.clone(),
            node: self.node_ref.clone(),
            frame: frame.clone(),
        };
        let mut step = CmpStep::new(self.display_name().to_string(), CmpStepTypeEnum::Single);
        step.node_name = self.instance.name().to_string();
        step.tag = self.node_ref.tag.clone();

        match self.instance.rollback(&component_context).await {
            Ok(()) => step.finish_rollback(true, None),
            Err(error) => step.finish_rollback(false, Some(error.to_string())),
        }
        if let Ok(mut rollback_steps) = ctx.inner.rollback_steps.lock() {
            rollback_steps.push(step);
        }
        Ok(())
    }
}
