use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

type Comparator<T> = dyn Fn(&T, &T) -> Ordering + Send + Sync;

/// 线程安全的有界优先级队列。
///
/// 队列达到容量上限后，仅当新元素的优先级高于当前最末元素时才替换它，
/// 因而始终保留排序结果中最靠前的 `capacity` 个元素。Java 版继承
/// `PriorityBlockingQueue`；Rust 使用短临界区 `Mutex<Vec<T>>`，避免暴露继承式
/// 容器 API，同时保留 `offer`、批量添加和有序快照语义。
///
/// 对应 Java: `com.yomahub.liteflow.util.BoundedPriorityBlockingQueue`。
pub struct BoundedPriorityBlockingQueue<T> {
    capacity: usize,
    comparator: Arc<Comparator<T>>,
    values: Mutex<Vec<T>>,
}

impl<T: Ord> BoundedPriorityBlockingQueue<T> {
    /// 使用类型的自然顺序创建队列。
    ///
    /// 参数 `capacity` 为最大保留元素数；传入 0 时队列拒绝所有元素。
    /// 对应 Java: `BoundedPriorityBlockingQueue#BoundedPriorityBlockingQueue(int)`。
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self::with_comparator(capacity, |left, right| left.cmp(right))
    }
}

impl<T> BoundedPriorityBlockingQueue<T> {
    /// 使用自定义比较器创建队列。
    ///
    /// 比较结果较小的元素排在前面；容量满时淘汰比较结果最大的末尾元素。
    /// 对应 Java: `BoundedPriorityBlockingQueue#BoundedPriorityBlockingQueue(int,Comparator)`。
    #[must_use]
    pub fn with_comparator(
        capacity: usize,
        comparator: impl Fn(&T, &T) -> Ordering + Send + Sync + 'static,
    ) -> Self {
        Self {
            capacity,
            comparator: Arc::new(comparator),
            values: Mutex::new(Vec::with_capacity(capacity)),
        }
    }

    /// 加入一个元素；队列满时仅保留优先级更高的新元素。
    ///
    /// 参数 `element` 为待加入元素；成功保留返回 `true`，容量为零或优先级不足
    /// 返回 `false`。对应 Java: `BoundedPriorityBlockingQueue#offer`。
    pub fn offer(&self, element: T) -> bool {
        if self.capacity == 0 {
            return false;
        }

        let mut values = self.values.lock().expect("有界优先级队列锁中毒");
        if values.len() < self.capacity {
            values.push(element);
            return true;
        }

        let last_index = values
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| (self.comparator)(left, right))
            .map(|(index, _)| index)
            .expect("容量非零且队列已满时必须存在末尾元素");
        if (self.comparator)(&element, &values[last_index]) != Ordering::Less {
            return false;
        }
        values[last_index] = element;
        true
    }

    /// 逐个加入集合中的所有元素，返回是否至少保留了一个新元素。
    ///
    /// 对应 Java: `BoundedPriorityBlockingQueue#addAll`。
    pub fn add_all<I>(&self, elements: I) -> bool
    where
        I: IntoIterator<Item = T>,
    {
        elements
            .into_iter()
            .fold(false, |changed, element| self.offer(element) || changed)
    }

    /// 返回当前元素数量。对应 Java 继承队列的 `size`。
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.lock().expect("有界优先级队列锁中毒").len()
    }

    /// 返回队列是否为空。对应 Java 继承队列的 `isEmpty`。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Clone> BoundedPriorityBlockingQueue<T> {
    /// 返回按原始比较器排序的独立快照。
    ///
    /// 快照不持有内部锁，可安全地用于报表迭代。
    /// 对应 Java: `BoundedPriorityBlockingQueue#toList`。
    #[must_use]
    pub fn to_list(&self) -> Vec<T> {
        let mut values = self.values.lock().expect("有界优先级队列锁中毒").clone();
        values.sort_by(|left, right| (self.comparator)(left, right));
        values
    }
}

impl<T: Clone> IntoIterator for &BoundedPriorityBlockingQueue<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    /// 返回有序快照迭代器。对应 Java 覆盖的 `iterator`。
    fn into_iter(self) -> Self::IntoIter {
        self.to_list().into_iter()
    }
}
