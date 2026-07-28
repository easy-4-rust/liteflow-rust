//! EL 通用修饰。

/// 可包裹任意表达式的通用修饰集合。
///
/// 对应 Java `RetryCondition`、`TimeoutCondition`、`ignoreError` 以及
/// 2.14+ Condition 级 `bind`。
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Mods {
    /// Condition 实例 id。
    pub id: Option<String>,
    /// Condition/Chain 标签。
    pub tag: Option<String>,
    /// Java 线程池实现类名。
    pub thread_pool: Option<String>,
    /// 最大重试次数。
    pub retry: Option<u32>,
    /// 允许重试的异常类型名。
    pub retry_for: Vec<String>,
    /// 最大等待毫秒数。
    pub max_wait_ms: Option<u64>,
    /// 是否忽略执行错误。
    pub ignore_error: bool,
    /// Condition 级绑定数据。
    pub bind: Vec<(String, String)>,
    /// 是否清除子节点上的同名绑定。
    pub bind_override: bool,
}
