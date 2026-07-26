//! 有限并发队列。
//!
//! 对应 Java: `com.yomahub.liteflow.util.LimitQueue`。

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

/// `offer` 超过上限时先移除最早元素，再加入新元素。
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
    pub fn offer(&self, element: E) {
        let mut queue = self.lock();
        if queue.len() >= self.limit {
            queue.pop_front();
        }
        queue.push_back(element);
    }

    /// 直接追加元素，不执行容量淘汰。
    ///
    /// Java 的 `add` 直接委托底层队列，与 `offer` 的行为不同。
    pub fn add(&self, element: E) {
        self.lock().push_back(element);
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

    /// 返回当前元素数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.lock().len()
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
}

impl<E: Clone> LimitQueue<E> {
    /// 克隆并返回队首元素；空队列返回 `None`。
    pub fn peek(&self) -> Option<E> {
        self.lock().front().cloned()
    }

    /// 返回队列当前快照，顺序为队首到队尾。
    pub fn queue(&self) -> Vec<E> {
        self.lock().iter().cloned().collect()
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
}
