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
#[derive(Debug, Clone, Default)]
pub struct Frame(pub Vec<(usize, Option<Value>)>);

impl Frame {
    pub fn root() -> Self {
        Self(Vec::new())
    }
    pub fn push(&self, index: usize, object: Option<Value>) -> Self {
        let mut v = self.0.clone();
        v.push((index, object));
        Self(v)
    }
    /// getLoopIndex()
    pub fn loop_index(&self) -> Option<usize> {
        self.0.last().map(|(i, _)| *i)
    }
    /// getLoopObject()
    pub fn loop_object(&self) -> Option<&Value> {
        self.0.last().and_then(|(_, o)| o.as_ref())
    }
    /// getLoopIndex(depth)，0 为最内层
    pub fn loop_index_at(&self, depth: usize) -> Option<usize> {
        self.0
            .len()
            .checked_sub(depth + 1)
            .and_then(|i| self.0.get(i))
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
}
