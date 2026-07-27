use liteflow_core::util::CopyOnWriteHashMap;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn java_named_collection_api_preserves_snapshot_and_clone_isolation() {
    let map = CopyOnWriteHashMap::new(HashMap::from([
        ("alpha".to_string(), 1),
        ("beta".to_string(), 2),
    ]));
    let snapshot = map.snapshot();
    let cloned = map.clone();

    assert_eq!(map.size(), 2);
    assert!(!map.is_empty());
    assert!(map.contains_key(&"alpha".to_string()));
    assert!(map.contains_value(&2));
    assert_eq!(map.get(&"beta".to_string()), Some(2));
    assert_eq!(
        map.key_set(),
        HashSet::from(["alpha".to_string(), "beta".to_string()])
    );
    assert_eq!(
        map.values().into_iter().collect::<HashSet<_>>(),
        [1, 2].into()
    );
    assert_eq!(
        map.entry_set(),
        HashSet::from([("alpha".to_string(), 1), ("beta".to_string(), 2)])
    );
    assert_eq!(map.to_string(), "{alpha=1, beta=2}");

    assert_eq!(map.put("alpha".to_string(), 10), Some(1));
    assert_eq!(map.put("gamma".to_string(), 3), None);
    map.put_all(&HashMap::from([
        ("delta".to_string(), 4),
        ("epsilon".to_string(), 5),
    ]));

    // 已取得的 view 快照和 clone 都不能观察到原对象后续发布的新视图。
    assert_eq!(snapshot.get("alpha"), Some(&1));
    assert!(!snapshot.contains_key("gamma"));
    assert_eq!(cloned.get(&"alpha".to_string()), Some(1));
    assert!(!cloned.contains_key(&"gamma".to_string()));

    assert_eq!(map.remove(&"beta".to_string()), Some(2));
    assert_eq!(map.remove(&"missing".to_string()), None);
    cloned.put("clone-only".to_string(), 99);
    assert!(!map.contains_key(&"clone-only".to_string()));

    map.clear();
    assert!(map.is_empty());
    assert_eq!(snapshot.len(), 2);
    assert_eq!(cloned.size(), 3);
}

#[test]
fn concurrent_puts_publish_complete_views_without_lost_updates() {
    const THREADS: usize = 8;
    const ITEMS_PER_THREAD: usize = 32;

    let map = Arc::new(CopyOnWriteHashMap::<usize, usize>::default());
    let barrier = Arc::new(Barrier::new(THREADS));
    let handles = (0..THREADS)
        .map(|thread_index| {
            let map = Arc::clone(&map);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                for item_index in 0..ITEMS_PER_THREAD {
                    let key = thread_index * ITEMS_PER_THREAD + item_index;
                    assert_eq!(map.put(key, key * 2), None);
                }
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().expect("并发写线程不应失败");
    }

    assert_eq!(map.size(), THREADS * ITEMS_PER_THREAD);
    for key in 0..THREADS * ITEMS_PER_THREAD {
        assert_eq!(map.get(&key), Some(key * 2));
    }
}
