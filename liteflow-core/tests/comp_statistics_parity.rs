use std::cmp::Ordering;
use std::thread;
use std::time::Duration;

use liteflow_core::CompStatistics;

/// 验证 CompStatistics 的 Java Bean API、null 比较和倒序时间排序语义。
#[test]
fn comp_statistics_exposes_java_accessors_and_newest_first_ordering() {
    let mut older = CompStatistics::new("demo.OlderComponent", 12);
    older.set_memory_spent(256);
    older.set_component_clazz_name("demo.RenamedComponent");
    older.set_time_spent(18);

    assert_eq!(older.get_component_clazz_name(), "demo.RenamedComponent");
    assert_eq!(older.get_time_spent(), 18);
    assert_eq!(older.get_memory_spent(), 256);
    assert!(older.get_record_time() > 0);
    assert_eq!(older.compare_to(None), Ordering::Greater);

    // Java 使用毫秒时间戳；跨过一个毫秒边界以验证严格的新记录优先顺序。
    thread::sleep(Duration::from_millis(2));
    let newer = CompStatistics::new("demo.NewerComponent", 6);
    assert_eq!(older.compare_to(Some(&newer)), Ordering::Greater);
    assert_eq!(newer.compare_to(Some(&older)), Ordering::Less);

    let mut records = vec![older, newer];
    records.sort();
    assert_eq!(records[0].get_component_clazz_name(), "demo.NewerComponent");
}
