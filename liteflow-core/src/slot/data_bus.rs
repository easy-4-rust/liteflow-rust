//! Slot 的分配、查询、扩容和回收中心。
//!
//! Java 使用 ConcurrentHashMap 保存 Slot、ConcurrentLinkedQueue 保存可用索引；
//! Rust 使用一个短临界区 `Mutex<DataBusState>` 原子维护两者，Slot 本身仍通过
//! `Arc` 跨异步任务共享。容量耗尽时按原容量的 1.75 倍扩容。
//!
//! 对应 Java: `com.yomahub.liteflow.slot.DataBus`。

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use super::{Slot, slot_lease::SlotLease};

const DEFAULT_SLOT_SIZE: usize = 1_024;

struct DataBusState {
    slots: HashMap<usize, Arc<Slot>>,
    available: VecDeque<usize>,
    current_index_max_value: usize,
}

impl DataBusState {
    fn with_capacity(slot_size: usize) -> Self {
        let slot_size = slot_size.max(1);
        Self {
            slots: HashMap::new(),
            available: (0..slot_size).collect(),
            current_index_max_value: slot_size,
        }
    }
}

/// 数据总线，负责 Slot 的分配和回收。
///
/// 对应 Java: `com.yomahub.liteflow.slot.DataBus`。
pub struct DataBus {
    state: Mutex<DataBusState>,
}

impl DataBus {
    fn load_instance() -> &'static Self {
        static INSTANCE: OnceLock<DataBus> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            state: Mutex::new(DataBusState::with_capacity(DEFAULT_SLOT_SIZE)),
        })
    }

    /// 初始化全局 Slot 池。
    ///
    /// 与 Java 一样，已有占用 Slot 时不会破坏当前执行；只有池尚未使用时才允许
    /// 用给定容量重建可用索引。`slot_size` 为 0 时按 1 处理。
    /// 对应 Java: `DataBus#init`。
    pub fn init(slot_size: usize) {
        let data_bus = Self::load_instance();
        let mut state = data_bus.state.lock().expect("DataBus 状态锁中毒");
        if state.slots.is_empty() {
            *state = DataBusState::with_capacity(slot_size);
        }
    }

    /// 把 Slot 放入全局池并返回分配到的索引。
    ///
    /// 容量不足时仅在持有状态锁期间扩容，确保并发分配不会得到重复索引。
    /// 对应 Java: `DataBus#offerIndex`。
    pub fn offer_slot(slot: Arc<Slot>) -> usize {
        let data_bus = Self::load_instance();
        let mut state = data_bus.state.lock().expect("DataBus 状态锁中毒");
        let slot_index = match state.available.pop_front() {
            Some(slot_index) => slot_index,
            None => {
                // Java 扩容到原容量的 1.75 倍；至少增加一个索引，避免小容量停滞。
                let old_max = state.current_index_max_value;
                let next_max = ((old_max as f64 * 1.75).round() as usize).max(old_max + 1);
                state.available.extend(old_max..next_max);
                state.current_index_max_value = next_max;
                state
                    .available
                    .pop_front()
                    .expect("DataBus 扩容后必须存在可用索引")
            }
        };
        state.slots.insert(slot_index, slot);
        slot_index
    }

    /// 按索引获取共享 Slot。
    ///
    /// 索引不存在或已经释放时返回 None。对应 Java: `DataBus#getSlot`。
    pub fn get_slot(slot_index: usize) -> Option<Arc<Slot>> {
        Self::load_instance()
            .state
            .lock()
            .expect("DataBus 状态锁中毒")
            .slots
            .get(&slot_index)
            .cloned()
    }

    /// 返回指定共享 Slot 当前在 DataBus 中的索引。
    ///
    /// Rust 内部持有 `Arc<Slot>`，此方法用于对齐 Java 组件可读取 slotIndex 的能力；
    /// Slot 已释放时返回 None。
    pub fn get_slot_index(slot: &Arc<Slot>) -> Option<usize> {
        Self::load_instance()
            .state
            .lock()
            .expect("DataBus 状态锁中毒")
            .slots
            .iter()
            .find_map(|(slot_index, current)| Arc::ptr_eq(slot, current).then_some(*slot_index))
    }

    /// 返回指定 Slot 中按名称登记的上下文 Bean。
    ///
    /// Java 返回 Tuple 列表；Rust 返回 `(名称, Arc<dyn Any>)`。
    /// 对应 Java: `DataBus#getContextBeanList`。
    pub fn get_context_bean_list(slot_index: usize) -> Vec<(String, Arc<dyn Any + Send + Sync>)> {
        Self::get_slot(slot_index)
            .map(|slot| {
                slot.beans
                    .iter()
                    .map(|entry| (entry.key().clone(), entry.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 释放指定 Slot，并把索引归还可用队列。
    ///
    /// 首次释放返回 true；索引不存在或已经释放返回 false，且不会重复放回队列。
    /// 对应 Java: `DataBus#releaseSlot`。
    pub fn release_slot(slot_index: usize) -> bool {
        let data_bus = Self::load_instance();
        let mut state = data_bus.state.lock().expect("DataBus 状态锁中毒");
        if state.slots.remove(&slot_index).is_none() {
            return false;
        }
        state.available.push_back(slot_index);
        true
    }

    /// 返回当前被占用的 Slot 数量。
    ///
    /// 对应 Java 公共计数器 `DataBus.OCCUPY_COUNT`。
    pub fn occupy_count() -> usize {
        Self::load_instance()
            .state
            .lock()
            .expect("DataBus 状态锁中毒")
            .slots
            .len()
    }

    /// 返回当前 Slot 池总容量。
    ///
    /// 用于 `MonitorBus#printStatistics` 输出 Java 的 `SLOT TOTAL SIZE`。
    #[must_use]
    pub fn total_size() -> usize {
        Self::load_instance()
            .state
            .lock()
            .expect("DataBus 状态锁中毒")
            .current_index_max_value
    }

    /// 为执行器创建自动回收的 Slot 租约。
    pub(crate) fn lease_slot(slot: Arc<Slot>) -> SlotLease {
        let slot_index = Self::offer_slot(slot.clone());
        SlotLease { slot_index, slot }
    }
}
