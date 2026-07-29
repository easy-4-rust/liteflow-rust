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
    /// Condition 级绑定数据。
    pub bind: Vec<(String, String)>,
    /// 需要清除子节点同名绑定的 key。
    ///
    /// Java 的 `override` 只属于当前一次 `bind(key, value, override)` 调用，
    /// 不能在多次 bind 合并后扩散到其他 key。
    pub bind_override_keys: Vec<String>,
}

impl Mods {
    /// 判断本次操作符是否会创建新的运行时包装 Condition。
    ///
    /// Java `RetryOperator` 与 `MaxWaitTimeOperator` 每调用一次都会创建新的
    /// `RetryCondition`/`TimeoutCondition`，因此不能像 id、tag、bind 等属性
    /// 操作符一样合并到已有对象。该方法只服务 Rust AST 组装，不对应 Java
    /// 公共 API。
    #[must_use]
    pub(crate) fn creates_wrapper_condition(&self) -> bool {
        self.retry.is_some() || self.max_wait_ms.is_some()
    }
}
