//! 对应 Java: com.yomahub.liteflow.lifecycle.impl.ChainCacheLifeCycle

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use dashmap::{DashMap, DashSet};

use crate::lifecycle::PostProcessChainExecuteLifeCycle;

fn lifecycle_instance() -> &'static OnceLock<Arc<ChainCacheLifeCycle>> {
    static INSTANCE: OnceLock<Arc<ChainCacheLifeCycle>> = OnceLock::new();
    &INSTANCE
}

/// Chain 最近使用缓存生命周期。
///
/// 超出容量时把最久未使用 Chain 标记为非活跃，并仅调用一次清理函数。
pub struct ChainCacheLifeCycle {
    capacity: usize,
    cache: DashMap<String, Arc<ChainState>>,
    order: Mutex<VecDeque<String>>,
    cleaned: DashSet<String>,
    clean_chain: Arc<dyn Fn(&str) + Send + Sync>,
}

/// Chain 在缓存中的活跃状态。
pub struct ChainState {
    active: AtomicBool,
}

impl ChainState {
    /// 创建活跃状态。对应 Java `newActiveState`。
    #[must_use]
    pub fn new_active_state() -> Self {
        Self {
            active: AtomicBool::new(true),
        }
    }

    /// 返回是否活跃。
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// 修改活跃状态。
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Release);
    }
}

impl ChainCacheLifeCycle {
    /// 创建指定容量与清理动作的生命周期。
    #[must_use]
    pub fn new(capacity: usize, clean_chain: Arc<dyn Fn(&str) + Send + Sync>) -> Self {
        Self {
            capacity: capacity.max(1),
            cache: DashMap::new(),
            order: Mutex::new(VecDeque::new()),
            cleaned: DashSet::new(),
            clean_chain,
        }
    }

    /// 初始化进程级生命周期实例；已初始化时返回 `false`。
    pub fn init_if_absent(capacity: usize, clean_chain: Arc<dyn Fn(&str) + Send + Sync>) -> bool {
        lifecycle_instance()
            .set(Arc::new(Self::new(capacity, clean_chain)))
            .is_ok()
    }

    /// 获取进程级生命周期实例。
    #[must_use]
    pub fn get_life_cycle() -> Option<Arc<Self>> {
        lifecycle_instance().get().cloned()
    }

    /// 返回 Chain 是否仍处于活跃缓存。
    #[must_use]
    pub fn is_active(&self, chain_id: &str) -> bool {
        self.cache
            .get(chain_id)
            .is_some_and(|state| state.is_active())
    }

    /// 返回 Chain 是否已经触发清理。
    #[must_use]
    pub fn is_cleaned(&self, chain_id: &str) -> bool {
        self.cleaned.contains(chain_id)
    }

    fn touch(&self, chain_id: &str) {
        let mut order = self.order.lock().expect("Chain 缓存顺序锁中毒");
        order.retain(|cached| cached != chain_id);
        order.push_back(chain_id.to_string());
        self.cache
            .entry(chain_id.to_string())
            .or_insert_with(|| Arc::new(ChainState::new_active_state()))
            .set_active(true);
        self.cleaned.remove(chain_id);

        while order.len() > self.capacity {
            if let Some(evicted) = order.pop_front() {
                if let Some((_, state)) = self.cache.remove(&evicted) {
                    state.set_active(false);
                }
                self.clean_once(&evicted);
            }
        }
    }

    fn clean_once(&self, chain_id: &str) {
        if self.cleaned.insert(chain_id.to_string()) {
            (self.clean_chain)(chain_id);
        }
    }
}

#[async_trait]
impl PostProcessChainExecuteLifeCycle for ChainCacheLifeCycle {
    /// 执行前记录 Chain 为活跃，并刷新 LRU 顺序。
    async fn post_process_before_chain_execute(&self, chain_id: &str) {
        self.touch(chain_id);
    }

    /// 执行后清理已被淘汰且尚未清理的 Chain。
    async fn post_process_after_chain_execute(&self, chain_id: &str) {
        if !self.is_active(chain_id) && !self.is_cleaned(chain_id) {
            self.clean_once(chain_id);
        }
    }
}
