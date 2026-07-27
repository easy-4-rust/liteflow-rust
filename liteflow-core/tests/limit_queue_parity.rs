//! Java `LimitQueue` 有限入队与集合委托语义测试。

use liteflow_core::util::LimitQueue;

#[test]
fn offer_evicts_but_add_and_add_all_keep_java_delegate_semantics() {
    let queue = LimitQueue::new(2);

    assert!(queue.offer(1));
    assert!(queue.offer(2));
    assert!(queue.offer(3));
    assert_eq!(queue.get_queue(), vec![2, 3]);
    assert_eq!(queue.get_limit(), 2);

    // Java add/addAll 直接委托 ConcurrentLinkedQueue，不受 limit 约束。
    assert!(queue.add(4));
    assert!(queue.add_all([5, 6]));
    assert_eq!(queue.size(), 5);
    assert_eq!(queue.get_queue(), vec![2, 3, 4, 5, 6]);
}

#[test]
fn java_collection_methods_operate_on_one_consistent_queue_snapshot() {
    let queue = LimitQueue::new(8);
    queue.add_all([1, 2, 3, 4, 5]);

    assert_eq!(queue.element(), Some(1));
    assert_eq!(queue.peek(), Some(1));
    assert!(queue.contains_all(&[1, 3, 5]));
    assert!(!queue.contains_all(&[1, 9]));
    assert_eq!(queue.iterator().collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
    assert_eq!(queue.to_array(), vec![1, 2, 3, 4, 5]);

    assert!(queue.remove_all(&[2, 4]));
    assert_eq!(queue.get_queue(), vec![1, 3, 5]);
    assert!(!queue.remove_all(&[8, 9]));

    assert!(queue.retain_all(&[1, 5]));
    assert_eq!(queue.to_array(), vec![1, 5]);
    assert!(!queue.retain_all(&[1, 5]));
}

#[test]
fn returned_queue_and_iterator_are_isolated_from_later_writes() {
    let queue = LimitQueue::new(3);
    queue.add_all(["a".to_string(), "b".to_string()]);

    let snapshot = queue.get_queue();
    let iterator = queue.iterator();
    queue.add("c".to_string());

    assert_eq!(snapshot, vec!["a", "b"]);
    assert_eq!(iterator.collect::<Vec<_>>(), vec!["a", "b"]);
    assert_eq!(queue.get_queue(), vec!["a", "b", "c"]);

    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(queue.element(), None);
}
