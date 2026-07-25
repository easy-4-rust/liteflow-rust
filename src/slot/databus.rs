//! 对应 DataBus：Slot 的持有与传递句柄。

use crate::flow::entity::cmp_step::CmpStep;
use crate::slot::slot::Slot;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(0);

/// 对应 DefaultRequestIdGenerator
pub fn gen_request_id() -> String {
    let seq = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{ts:x}{seq:04x}")
}

/// 对应 Java Node 上的 loopIndexTL / loopObjectTL 栈（按执行路径传递）
/// 以及 2.16 起 Slot.conditionStack 上 Condition.bindData 的查找路径：
/// Rust 端按执行路径（Frame clone）传递 bind 键值栈，语义等价于
/// Java 的「从 condition 栈顶向下遍历查找 bindData」。
#[derive(Debug, Clone, Default)]
pub struct Frame {
    /// (loopIndex, loopObject) 栈
    pub loops: Vec<(usize, Option<Value>)>,
    /// Condition 级 bind 键值栈（内层在后，查找时从后往前，即栈顶优先）
    pub binds: Vec<(String, String)>,
}

impl Frame {
    pub fn root() -> Self {
        Self::default()
    }
    pub fn push(&self, index: usize, object: Option<Value>) -> Self {
        let mut f = self.clone();
        f.loops.push((index, object));
        f
    }
    /// 压入 Condition 级 bind 键值（对应 Condition.putBindData + conditionStack）
    pub fn push_bind(&self, pairs: &[(String, String)]) -> Self {
        if pairs.is_empty() {
            return self.clone();
        }
        let mut f = self.clone();
        f.binds.extend(pairs.iter().cloned());
        f
    }
    /// 从栈顶向下查找 bindData（对应 NodeComponent.getBindData 的 condition 栈查找）
    pub fn find_bind(&self, key: &str) -> Option<&str> {
        self.binds
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    /// getLoopIndex()
    pub fn loop_index(&self) -> Option<usize> {
        self.loops.last().map(|(i, _)| *i)
    }
    /// getLoopObject()
    pub fn loop_object(&self) -> Option<&Value> {
        self.loops.last().and_then(|(_, o)| o.as_ref())
    }
    /// getLoopIndex(depth)，0 为最内层
    pub fn loop_index_at(&self, depth: usize) -> Option<usize> {
        self.loops
            .len()
            .checked_sub(depth + 1)
            .and_then(|i| self.loops.get(i))
            .map(|(i, _)| *i)
    }
}

/// DataBus 句柄：等价于 Java 中按 slotIndex 取 Slot
#[derive(Clone)]
pub struct Ctx {
    pub inner: Arc<Slot>,
}

impl Ctx {
    pub fn new(inner: Arc<Slot>) -> Self {
        Self { inner }
    }
    pub fn record_step(&self, step: CmpStep) {
        if let Ok(mut s) = self.inner.steps.lock() {
            s.push(step);
        }
    }
    pub fn set_exception(&self, e: &str) {
        if let Ok(mut slot) = self.inner.exception.lock() {
            *slot = Some(e.to_string());
        }
    }
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
