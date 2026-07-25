//! 对应 flow.element.Node：包装组件实例的可执行节点。
//! execute() 对齐 Java Node.execute → processFlow 语义：
//! isAccess → beforeProcess → process → afterProcess，
//! 异常时 onError → isContinueOnError 决定是否吞掉，全部记入 CmpStep。

use crate::aop::CmpAroundAspect;
use crate::core::node_component::NodeComponent;
use crate::monitor::MonitorBus;
use crate::el::NodeRef;
use crate::enums::CmpStepTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::{CmpContext, Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 节点级横切钩子（AOP + 监控），构建期由 builder 注入
#[derive(Clone, Default)]
pub struct NodeHooks {
    pub aspects: Vec<Arc<dyn CmpAroundAspect>>,
    pub monitor: Option<Arc<MonitorBus>>,
}

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
}

#[async_trait]
impl Executable for Node {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
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

        let mut step = CmpStep::new(self.display_name().to_string(), CmpStepTypeEnum::Node);
        step.tag = self.node_ref.tag.clone();
        step.node_name = self.instance.name().to_string();

        // 全局切面 before（对应 CmpAroundAspect）
        for aspect in &self.hooks.aspects {
            aspect.before(&cctx).await;
        }

        // beforeProcess
        if let Err(e) = self.instance.before_process(&cctx).await {
            self.instance.on_error(&cctx, &e).await;
            self.instance.after_process(&cctx).await;
            step.finish(false, Some(e.to_string()));
            ctx.record_step(step);
            if self.instance.is_continue_on_error() {
                ctx.set_exception(&e.to_string());
                return Ok(Value::Null);
            }
            return Err(LiteflowError::NodeExec {
                node: self.display_name().to_string(),
                msg: e.to_string(),
            });
        }

        let result = self.instance.process(&cctx).await;
        self.instance.after_process(&cctx).await;

        // 全局切面 after / on_error
        for aspect in &self.hooks.aspects {
            aspect.after(&cctx).await;
            if let Err(e) = &result {
                aspect.on_error(&cctx, e).await;
            }
        }

        match result {
            Ok(v) => {
                step.finish(true, None);
                if let Some(m) = &self.hooks.monitor {
                    m.record(self.display_name(), step.time_spent.unwrap_or_default(), true);
                }
                ctx.record_step(step);
                // setIsEnd(true) 语义
                if ctx.is_ended() {
                    return Err(LiteflowError::ChainEnd);
                }
                Ok(v)
            }
            Err(LiteflowError::ChainEnd) => {
                step.finish(true, None);
                ctx.record_step(step);
                Err(LiteflowError::ChainEnd)
            }
            Err(e) => {
                self.instance.on_error(&cctx, &e).await;
                ctx.set_exception(&e.to_string());
                step.finish(false, Some(e.to_string()));
                if let Some(m) = &self.hooks.monitor {
                    m.record(self.display_name(), step.time_spent.unwrap_or_default(), false);
                }
                ctx.record_step(step);
                if self.instance.is_continue_on_error() {
                    return Ok(Value::Null);
                }
                Err(LiteflowError::NodeExec {
                    node: self.display_name().to_string(),
                    msg: e.to_string(),
                })
            }
        }
    }

    fn id(&self) -> &str {
        self.node_ref.display()
    }

    fn tag(&self) -> Option<&str> {
        self.node_ref.tag.as_deref()
    }
}
