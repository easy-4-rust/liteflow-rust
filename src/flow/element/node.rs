//! 对应 flow.element.Node：组件引用与执行包装。

use crate::core::node_component::NodeComponent;
use crate::el::NodeRef;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{CmpContext, Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 对应 NodeHook / 全局 AOP + 监控
#[derive(Clone, Default)]
pub struct NodeHooks {
    pub aspects: Vec<Arc<dyn crate::aop::CmpAroundAspect>>,
    pub monitor: Option<crate::monitor::MonitorBus>,
}

pub struct Node {
    node_ref: NodeRef,
    instance: Arc<dyn NodeComponent>,
    hooks: NodeHooks,
    /// nodeInstanceId（对应 NodeInstanceIdManageSpi，默认为 chainId_nodeId_occurrence）
    node_instance_id: String,
}

impl Node {
    pub fn new(
        node_ref: NodeRef,
        instance: Arc<dyn NodeComponent>,
        hooks: NodeHooks,
        node_instance_id: String,
    ) -> Self {
        Self { node_ref, instance, hooks, node_instance_id }
    }

    /// getNodeInstanceId()
    pub fn node_instance_id(&self) -> &str {
        &self.node_instance_id
    }

    /// processFlow 语义：access 检查 → beforeProcess → process → afterProcess → onError
    async fn process_flow(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let cctx = CmpContext {
            inner: ctx.inner.clone(),
            node: self.node_ref.clone(),
            frame: frame.clone(),
        };

        if !self.instance.is_access(&cctx) {
            return Ok(Value::Null);
        }

        // AOP before + Monitor record start
        for a in &self.hooks.aspects {
            a.before(&cctx).await;
        }
        let start = std::time::Instant::now();

        self.instance.before_process(&cctx).await;
        let result = self.instance.process(&cctx).await;
        let elapsed = start.elapsed().as_millis() as u64;

        match result {
            Ok(v) => {
                self.instance.after_process(&cctx).await;
                for a in &self.hooks.aspects {
                    a.after(&cctx).await;
                }
                if let Some(m) = &self.hooks.monitor {
                    m.record(self.node_instance_id(), true, elapsed);
                }
                Ok(v)
            }
            Err(e) => {
                self.instance.on_error(&cctx, &e).await;
                for a in &self.hooks.aspects {
                    a.on_error(&cctx, &e).await;
                }
                if let Some(m) = &self.hooks.monitor {
                    m.record(self.node_instance_id(), false, elapsed);
                }
                if self.instance.is_continue_on_error(&cctx) {
                    Ok(Value::Null)
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[async_trait]
impl Executable for Node {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        if ctx.is_ended() {
            return Err(LiteflowError::ChainEnd);
        }
        self.process_flow(ctx, frame).await
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
