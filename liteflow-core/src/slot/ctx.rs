//! Rust 异步执行链中的 Slot 持有与传递句柄。
//!
//! Java 通过整数 slotIndex 从 DataBus 反查 Slot；Rust 内部执行路径使用 `Arc<Slot>`
//! 避免跨 await 期间索引失效。真正的 DataBus 分配和回收逻辑位于 `data_bus.rs`。

use crate::core::NodeComponent;
use crate::enums::CmpStepTypeEnum;
use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::cmp_context::CmpContext;
use crate::slot::slot::Slot;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Rust 执行上下文句柄，持有本次执行的共享 Slot。
#[derive(Clone)]
pub struct Ctx {
    pub inner: Arc<Slot>,
}

impl Ctx {
    /// 由共享 Slot 创建执行上下文。
    pub fn new(inner: Arc<Slot>) -> Self {
        Self { inner }
    }

    /// 向共享 Slot 追加一条组件执行步骤。
    ///
    /// 参数 `step` 包含节点、耗时、结果和异常等快照；承接 Java
    /// `Slot#addStep` 的内部调用。
    pub fn record_step(&self, step: CmpStep) {
        self.inner.add_step(step);
    }
    /// 登记失败时需要逆序补偿的节点执行记录。
    ///
    /// 对应 Java `CmpStep#setInstance/setRefNode` 保存的回滚目标。Rust 不把运行时
    /// trait object 暴露到公共 `CmpStep`，而是保存在 Slot 的内部回滚队列。
    pub fn register_rollback(
        &self,
        node_instance_id: String,
        component: Arc<dyn NodeComponent>,
        context: CmpContext,
    ) {
        if let Ok(mut items) = self.inner.rollback_items.lock() {
            items.push((node_instance_id, component, context.node, context.frame));
        }
    }

    /// 按执行记录逆序回滚，且同一节点实例只执行一次。
    ///
    /// 对应 Java `FlowExecutor#doExecute` 中对 executeSteps 的
    /// `descendingIterator()` 遍历，以及 `NodeComponent#doRollback` 的去重行为。
    /// 回滚异常只写入 rollbackSteps，不覆盖链路的原始异常。
    pub async fn rollback(&self) {
        let items = self
            .inner
            .rollback_items
            .lock()
            .map(|items| items.clone())
            .unwrap_or_default();
        let mut rolled_back = HashSet::new();

        for (node_instance_id, component, node, frame) in items.into_iter().rev() {
            if !rolled_back.insert(node_instance_id.clone()) {
                continue;
            }

            let context = CmpContext {
                inner: self.inner.clone(),
                node,
                frame,
            };
            let mut step = CmpStep::new(
                context.node.display().to_string(),
                component.name(),
                CmpStepTypeEnum::Single,
            );
            step.node_instance_id = Some(node_instance_id);
            step.tag = context.node.tag.clone();
            step.set_instance(component.clone());
            match component.do_rollback(&context).await {
                Ok(()) => step.finish_rollback(true, None),
                Err(error) => step.finish_rollback(false, Some(error.to_string())),
            }
            self.inner.add_rollback_step(step);
        }
    }
    /// 将异常文本写入共享 Slot。
    ///
    /// 参数 `e` 是当前 Chain 或 Condition 失败原因。对应 Java:
    /// `Slot#setException` 的执行期调用。
    pub fn set_exception(&self, e: &str) {
        if let Ok(mut slot) = self.inner.exception.lock() {
            *slot = Some(e.to_string());
        }
    }

    /// 返回当前 Slot 是否收到主动结束标记。
    ///
    /// true 对应组件调用 Java `NodeComponent#setIsEnd(true)` 后的状态。
    #[must_use]
    pub fn is_ended(&self) -> bool {
        self.inner.ended.load(Ordering::Relaxed)
    }
    /// conversationId（2.15+）
    pub fn conversation_id(&self) -> Option<&str> {
        self.inner.conversation_id.as_deref()
    }
    /// Slot.setAttachment（2.15+）
    pub fn set_attachment<T: std::any::Any + Send + Sync>(&self, key: impl Into<String>, value: T) {
        self.inner.set_attachment(key, value);
    }
    /// Slot.getAttachment
    pub fn get_attachment<T: std::any::Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.inner.get_attachment(key)
    }
    /// Slot.hasAttachment
    pub fn has_attachment(&self, key: &str) -> bool {
        self.inner.has_attachment(key)
    }
    /// Slot.removeAttachment
    pub fn remove_attachment(&self, key: &str) {
        self.inner.remove_attachment(key);
    }
}
