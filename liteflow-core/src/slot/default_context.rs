//! LiteFlow 默认上下文 Bean。
//!
//! 对应 Java: `com.yomahub.liteflow.slot.DefaultContext`。

use std::collections::HashMap;

use dashmap::DashMap;
use serde_json::Value;

use crate::exception::NullParamException;

/// 提供线程安全、弱类型的默认业务上下文。
///
/// Java Javadoc 建议正式业务优先定义强类型上下文 Bean；本对象用于无需额外类型
/// 定义的简单场景。Rust 使用 `serde_json::Value` 映射 Java `Object`，并以
/// `DashMap` 对齐 `ConcurrentHashMap` 的并发读写语义。
///
/// 对应 Java: `com.yomahub.liteflow.slot.DefaultContext`。
#[derive(Debug, Default)]
pub struct DefaultContext {
    data_map: DashMap<String, Value>,
}

impl DefaultContext {
    /// 创建空的默认上下文。
    ///
    /// - 返回：不包含任何键值的线程安全上下文。
    ///
    /// 对应 Java: `DefaultContext#DefaultContext`。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 判断上下文是否包含指定键。
    ///
    /// - `key`：待查询的数据键。
    /// - 返回：键存在时为 `true`，否则为 `false`。
    ///
    /// 对应 Java: `DefaultContext#hasData`。
    #[must_use]
    pub fn has_data(&self, key: &str) -> bool {
        self.data_map.contains_key(key)
    }

    /// 获取指定键对应的 serde 值。
    ///
    /// - `key`：待查询的数据键。
    /// - 返回：键存在时返回值快照，不存在时返回 `None`。
    ///
    /// Java 的泛型强制转换由调用方承担；Rust 返回拥有型 `Value`，避免并发锁守卫
    /// 泄漏到调用方。对应 Java: `DefaultContext#getData`。
    #[must_use]
    pub fn get_data(&self, key: &str) -> Option<Value> {
        self.data_map.get(key).map(|entry| entry.value().clone())
    }

    /// 写入指定键的数据。
    ///
    /// - `key`：数据键。
    /// - `value`：待写入的 serde 值；`Value::Null` 对应 Java `null`，会被拒绝。
    /// - 返回：写入成功返回 `Ok(())`；空值返回 `NullParamException`。
    ///
    /// Java `ConcurrentHashMap` 不接受 null，本方法保留
    /// `data can't accept null param` 的异常语义。对应 Java:
    /// `DefaultContext#setData`。
    pub fn set_data(&self, key: impl Into<String>, value: Value) -> Result<(), NullParamException> {
        if value.is_null() {
            return Err(NullParamException::new("data can't accept null param"));
        }

        // DashMap 在分片写锁内完成替换，使并发读者不会观察到中间状态。
        self.data_map.insert(key.into(), value);
        Ok(())
    }

    /// 返回当前数据映射的拥有型快照。
    ///
    /// - 返回：调用时刻全部键值的 `HashMap` 快照。
    ///
    /// Java 返回活动的 `ConcurrentHashMap`；Rust 不向外暴露锁守卫，因此返回安全快照。
    /// 后续 `set_data` 不会修改已经取得的快照。对应 Java:
    /// `DefaultContext#getDataMap`。
    #[must_use]
    pub fn get_data_map(&self) -> HashMap<String, Value> {
        self.data_map
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}
