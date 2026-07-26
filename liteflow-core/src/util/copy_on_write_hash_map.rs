//! 对应 Java: com.yomahub.liteflow.util.CopyOnWriteHashMap

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

/// 写时复制 HashMap。
///
/// 读取仅克隆 `Arc` 快照；每次写入先复制完整 Map，再原子替换当前视图。
pub struct CopyOnWriteHashMap<K, V> {
    view: RwLock<Arc<HashMap<K, V>>>,
}

impl<K, V> Default for CopyOnWriteHashMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self {
            view: RwLock::new(Arc::new(HashMap::new())),
        }
    }
}

impl<K, V> Clone for CopyOnWriteHashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            view: RwLock::new(Arc::new(self.snapshot().as_ref().clone())),
        }
    }
}

impl<K, V> CopyOnWriteHashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// 从已有 Map 创建独立视图。
    #[must_use]
    pub fn new(map: HashMap<K, V>) -> Self {
        Self {
            view: RwLock::new(Arc::new(map)),
        }
    }

    /// 返回当前不可变快照；后续写入不影响该快照。
    #[must_use]
    pub fn snapshot(&self) -> Arc<HashMap<K, V>> {
        self.view.read().expect("写时复制 Map 读锁中毒").clone()
    }

    /// 返回元素数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.view.read().expect("写时复制 Map 读锁中毒").len()
    }

    /// 返回 Map 是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 获取值的克隆。
    #[must_use]
    pub fn get(&self, key: &K) -> Option<V> {
        self.view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .get(key)
            .cloned()
    }

    /// 插入键值并返回旧值。
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let mut guard = self.view.write().expect("写时复制 Map 写锁中毒");
        let mut next = guard.as_ref().clone();
        let previous = next.insert(key, value);
        *guard = Arc::new(next);
        previous
    }

    /// 移除键并返回旧值。
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut guard = self.view.write().expect("写时复制 Map 写锁中毒");
        let mut next = guard.as_ref().clone();
        let previous = next.remove(key);
        *guard = Arc::new(next);
        previous
    }

    /// 批量写入键值。
    pub fn extend(&self, values: impl IntoIterator<Item = (K, V)>) {
        let mut guard = self.view.write().expect("写时复制 Map 写锁中毒");
        let mut next = guard.as_ref().clone();
        next.extend(values);
        *guard = Arc::new(next);
    }

    /// 清空 Map 并发布新视图。
    pub fn clear(&self) {
        *self.view.write().expect("写时复制 Map 写锁中毒") = Arc::new(HashMap::new());
    }
}
