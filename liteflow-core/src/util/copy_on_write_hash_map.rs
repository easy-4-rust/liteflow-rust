//! 对应 Java: com.yomahub.liteflow.util.CopyOnWriteHashMap

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

/// 写时复制 HashMap。
///
/// 读取始终访问当前不可变快照；克隆对象后，原对象或克隆对象上的任意写操作都会
/// 先复制各自状态，再发布新的视图，因此双方后续修改互不影响。
///
/// 对应 Java: `com.yomahub.liteflow.util.CopyOnWriteHashMap`。
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
    /// 从已有 Map 创建独立写时复制视图。
    ///
    /// - `map`: 初始键值集合；构造时完整复制，后续修改不会影响调用方。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#CopyOnWriteHashMap`。
    #[must_use]
    pub fn new(map: HashMap<K, V>) -> Self {
        Self {
            view: RwLock::new(Arc::new(map)),
        }
    }

    /// 返回当前不可变快照；后续写入不会改变已经取得的快照。
    ///
    /// 这是 Rust 侧只读快照入口，用于实现 Java `view` 的一次性读取语义。
    #[must_use]
    pub fn snapshot(&self) -> Arc<HashMap<K, V>> {
        self.view.read().expect("写时复制 Map 读锁中毒").clone()
    }

    /// 返回元素数量。对应 Rust 容器惯用命名。
    #[must_use]
    pub fn len(&self) -> usize {
        self.view.read().expect("写时复制 Map 读锁中毒").len()
    }

    /// 返回元素数量。
    ///
    /// # 返回
    /// 当前已发布视图中的键值数量。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#size`。
    #[must_use]
    pub fn size(&self) -> usize {
        self.len()
    }

    /// 判断 Map 是否为空。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#isEmpty`。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 判断当前视图是否包含指定键。
    ///
    /// - `key`: 待查找的键。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#containsKey`。
    #[must_use]
    pub fn contains_key(&self, key: &K) -> bool {
        self.view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .contains_key(key)
    }

    /// 判断当前视图是否包含指定值。
    ///
    /// - `value`: 待比较的值。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#containsValue`。
    #[must_use]
    pub fn contains_value(&self, value: &V) -> bool
    where
        V: PartialEq,
    {
        self.view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .values()
            .any(|candidate| candidate == value)
    }

    /// 获取指定键对应值的克隆。
    ///
    /// - `key`: 待查找的键。
    /// - 返回：键存在时返回值快照，否则返回 `None`。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#get`。
    #[must_use]
    pub fn get(&self, key: &K) -> Option<V> {
        self.view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .get(key)
            .cloned()
    }

    /// 返回当前视图的键集合快照。
    ///
    /// 后续写操作发布新视图时，不会改变已经返回的集合。
    /// 对应 Java: `CopyOnWriteHashMap#keySet`。
    #[must_use]
    pub fn key_set(&self) -> HashSet<K> {
        self.view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .keys()
            .cloned()
            .collect()
    }

    /// 返回当前视图的值集合快照。
    ///
    /// Java 返回 `Collection<V>`，Rust 用 `Vec<V>` 保留重复值。
    /// 对应 Java: `CopyOnWriteHashMap#values`。
    #[must_use]
    pub fn values(&self) -> Vec<V> {
        self.view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .values()
            .cloned()
            .collect()
    }

    /// 返回当前视图的键值条目集合快照。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#entrySet`。
    #[must_use]
    pub fn entry_set(&self) -> HashSet<(K, V)>
    where
        V: Eq + Hash,
    {
        self.view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    /// 按 Java Map 形式生成字符串。
    ///
    /// 条目按渲染后的文本排序，仅用于消除 Rust `HashMap` 随机迭代顺序，不改变
    /// Map 的键值语义。对应 Java: `CopyOnWriteHashMap#toString`。
    #[must_use]
    pub fn to_string(&self) -> String
    where
        K: Display,
        V: Display,
    {
        let mut entries = self
            .view
            .read()
            .expect("写时复制 Map 读锁中毒")
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>();
        entries.sort();
        format!("{{{}}}", entries.join(", "))
    }

    /// 克隆当前视图为一个后续写入相互隔离的新对象。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#clone`。
    #[must_use]
    pub fn clone(&self) -> Self {
        <Self as Clone>::clone(self)
    }

    /// 写入键值并返回写入前的旧值。
    ///
    /// - `key`: 要写入的键。
    /// - `value`: 要写入的值。
    ///
    /// 写入时先复制当前完整视图，再一次性替换已发布视图。
    /// 对应 Java: `CopyOnWriteHashMap#put`。
    pub fn put(&self, key: K, value: V) -> Option<V> {
        self.insert(key, value)
    }

    /// 插入键值并返回旧值。
    ///
    /// 这是 `put` 的 Rust 容器惯用别名。
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let mut guard = self.view.write().expect("写时复制 Map 写锁中毒");
        let mut next = guard.as_ref().clone();
        let previous = next.insert(key, value);
        *guard = Arc::new(next);
        previous
    }

    /// 移除指定键并返回旧值。
    ///
    /// - `key`: 要移除的键。
    ///
    /// 对应 Java: `CopyOnWriteHashMap#remove`。
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut guard = self.view.write().expect("写时复制 Map 写锁中毒");
        let mut next = guard.as_ref().clone();
        let previous = next.remove(key);
        *guard = Arc::new(next);
        previous
    }

    /// 批量复制并写入另一个 Map 的所有键值。
    ///
    /// - `t`: Java `putAll` 的输入 Map。
    ///
    /// 整批数据只复制和发布一次，避免逐项写入产生多个中间视图。
    /// 对应 Java: `CopyOnWriteHashMap#putAll`。
    pub fn put_all(&self, t: &HashMap<K, V>) {
        self.extend(t.iter().map(|(key, value)| (key.clone(), value.clone())));
    }

    /// 批量写入键值。
    ///
    /// 这是 `put_all` 的 Rust `IntoIterator` 扩展入口。
    pub fn extend(&self, values: impl IntoIterator<Item = (K, V)>) {
        let mut guard = self.view.write().expect("写时复制 Map 写锁中毒");
        let mut next = guard.as_ref().clone();
        next.extend(values);
        *guard = Arc::new(next);
    }

    /// 清空 Map 并发布全新的空视图。
    ///
    /// 已经取得的快照和克隆对象不受影响。
    /// 对应 Java: `CopyOnWriteHashMap#clear`。
    pub fn clear(&self) {
        *self.view.write().expect("写时复制 Map 写锁中毒") = Arc::new(HashMap::new());
    }
}
