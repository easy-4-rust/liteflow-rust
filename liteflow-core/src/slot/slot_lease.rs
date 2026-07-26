//! DataBus Slot 的异步取消安全租约。
//!
//! 这是 Rust 所有权模型适配对象：Java 在 FlowExecutor 的 finally 中释放索引；
//! Rust 通过 Drop 保证 future 被取消、提前返回或发生 panic 展开时仍归还索引。

use std::sync::Arc;

use super::{DataBus, Slot};

/// 自动回收 DataBus 索引的内部租约。
pub(crate) struct SlotLease {
    pub(crate) slot_index: usize,
    pub(crate) slot: Arc<Slot>,
}

impl SlotLease {
    /// 返回租约持有的共享 Slot。
    pub(crate) fn slot(&self) -> Arc<Slot> {
        self.slot.clone()
    }
}

impl Drop for SlotLease {
    fn drop(&mut self) {
        DataBus::release_slot(self.slot_index);
    }
}
