//! 对应 Java: com.yomahub.liteflow.annotation.AnnoUtil

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn annotation_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 过程宏展开期的注解元数据缓存。
///
/// Rust 不在运行期反射属性；缓存发生在编译期，并以“被注解对象 + 注解类型”
/// 作为稳定键，对齐 Java `AnnoUtil#getAnnotation` 的复用语义。
pub(crate) struct AnnoUtil;

impl AnnoUtil {
    /// 获取已缓存值，未命中时调用解析函数并写入缓存。
    pub(crate) fn get_annotation(
        annotated_element: &str,
        annotation_type: &str,
        parse: impl FnOnce() -> String,
    ) -> String {
        let cache_key = format!("{annotated_element}-{annotation_type}");
        let mut cache = annotation_cache().lock().expect("注解缓存锁中毒");
        cache.entry(cache_key).or_insert_with(parse).clone()
    }
}
