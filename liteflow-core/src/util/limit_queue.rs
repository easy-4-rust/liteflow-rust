//! 有限并发队列。
//!
//! 对应 Java: `com.yomahub.liteflow.util.LimitQueue`。

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

/// `offer` 超过上限时先移除最早元素，再加入新元素。
///
/// Java 的 `add/addAll` 刻意直接委托底层队列，因此不会执行容量淘汰；Rust 保留
/// 这一差异。内部使用 `Mutex<VecDeque<E>>` 映射 `ConcurrentLinkedQueue`，
/// 每个复合操作只持有一次锁。对应 Java:
/// `com.yomahub.liteflow.util.LimitQueue`。
pub struct LimitQueue<E> {
    limit: usize,
    queue: Mutex<VecDeque<E>>,
}

impl<E> LimitQueue<E> {
    /// 创建指定容量上限的队列。
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            queue: Mutex::new(VecDeque::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, VecDeque<E>> {
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// 按有限队列语义入队；满时先弹出队首。
    ///
    /// 对应 Java: `LimitQueue#offer`。
    pub fn offer(&self, element: E) -> bool {
        let mut queue = self.lock();
        if queue.len() >= self.limit {
            queue.pop_front();
        }
        queue.push_back(element);
        true
    }

    /// 直接追加元素，不执行容量淘汰。
    ///
    /// Java 的 `add` 直接委托底层队列，与 `offer` 的行为不同。
    pub fn add(&self, element: E) -> bool {
        self.lock().push_back(element);
        true
    }

    /// 弹出并返回队首元素；空队列返回 `None`。
    pub fn poll(&self) -> Option<E> {
        self.lock().pop_front()
    }

    /// 返回容量上限。
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// 返回实例化时指定的容量上限。
    ///
    /// 返回值只约束 `offer`，不约束 Java 直接委托语义的 `add/addAll`。对应 Java:
    /// `LimitQueue#getLimit`。
    #[must_use]
    pub fn get_limit(&self) -> usize {
        self.limit()
    }

    /// 返回当前元素数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// 返回当前元素数量。
    ///
    /// 该数量可能大于 limit，因为 Java `add/addAll` 不触发淘汰。对应 Java:
    /// `LimitQueue#size`。
    #[must_use]
    pub fn size(&self) -> usize {
        self.len()
    }

    /// 判断队列是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// 清空队列。
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// 直接批量追加元素，不执行容量淘汰。
    ///
    /// 参数 `elements` 对应 Java Collection；至少追加一个元素时返回 `true`。
    /// 对应 Java: `LimitQueue#addAll`。
    pub fn add_all<I>(&self, elements: I) -> bool
    where
        I: IntoIterator<Item = E>,
    {
        let mut queue = self.lock();
        let original_len = queue.len();
        queue.extend(elements);
        queue.len() != original_len
    }
}

impl<E: Clone> LimitQueue<E> {
    /// 克隆并返回队首元素；空队列返回 `None`。
    pub fn peek(&self) -> Option<E> {
        self.lock().front().cloned()
    }

    /// 返回队首元素但不移除。
    ///
    /// Java 空队列会抛出 `NoSuchElementException`；Rust 用 `Option` 显式表达空值。
    /// 对应 Java: `LimitQueue#element`。
    #[must_use]
    pub fn element(&self) -> Option<E> {
        self.peek()
    }

    /// 返回队列当前快照，顺序为队首到队尾。
    pub fn queue(&self) -> Vec<E> {
        self.lock().iter().cloned().collect()
    }

    /// 返回底层队列的独立有序快照。
    ///
    /// Java 返回可变 Queue 引用；Rust 不把锁保护的容器逸出临界区，调用方修改
    /// 快照不会影响原队列。对应 Java: `LimitQueue#getQueue`。
    #[must_use]
    pub fn get_queue(&self) -> Vec<E> {
        self.queue()
    }

    /// 返回队首到队尾的快照迭代器。
    ///
    /// 迭代期间后续并发写入不会改变当前迭代结果。对应 Java:
    /// `LimitQueue#iterator`。
    pub fn iterator(&self) -> std::vec::IntoIter<E> {
        self.queue().into_iter()
    }

    /// 返回队首到队尾的数组快照。
    ///
    /// Java 两个 `toArray` 重载统一映射为 Rust `Vec<E>`。对应 Java:
    /// `LimitQueue#toArray`。
    #[must_use]
    pub fn to_array(&self) -> Vec<E> {
        self.queue()
    }
}

impl<E: PartialEq> LimitQueue<E> {
    /// 删除第一个匹配元素并返回是否删除成功。
    pub fn remove(&self, element: &E) -> bool {
        let mut queue = self.lock();
        let Some(index) = queue.iter().position(|item| item == element) else {
            return false;
        };
        queue.remove(index);
        true
    }

    /// 判断队列是否包含指定元素。
    #[must_use]
    pub fn contains(&self, element: &E) -> bool {
        self.lock().contains(element)
    }

    /// 判断是否包含给定集合中的全部元素。
    ///
    /// 空集合返回 `true`。参数 `elements` 对应 Java Collection。对应 Java:
    /// `LimitQueue#containsAll`。
    #[must_use]
    pub fn contains_all(&self, elements: &[E]) -> bool {
        let queue = self.lock();
        elements.iter().all(|element| queue.contains(element))
    }

    /// 删除给定集合中出现的全部元素。
    ///
    /// 至少删除一个元素时返回 `true`。对应 Java: `LimitQueue#removeAll`。
    pub fn remove_all(&self, elements: &[E]) -> bool {
        let mut queue = self.lock();
        let original_len = queue.len();
        queue.retain(|item| !elements.contains(item));
        queue.len() != original_len
    }

    /// 只保留给定集合中出现的元素。
    ///
    /// 队列内容发生变化时返回 `true`。对应 Java: `LimitQueue#retainAll`。
    pub fn retain_all(&self, elements: &[E]) -> bool {
        let mut queue = self.lock();
        let original_len = queue.len();
        queue.retain(|item| elements.contains(item));
        queue.len() != original_len
    }
}
