//! 对应 2.14+ 的 Condition 级 bind（`THEN(...).bind(k, v[, override])`）。
//! Java 把 bindData 直接存在 Condition 对象上，执行时随 conditionStack 下传；
//! Rust 端用包装 Condition 持有 bind 键值，execute 时压入 Frame.bind 栈，
//! 查找语义与 Java 的「condition 栈顶向下遍历」一致。

use crate::exception::LFResult;
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct BindWrapperCondition {
    inner: Arc<dyn Executable>,
    bind_data: Vec<(String, String)>,
    id: Option<String>,
    tag: Option<String>,
    /// Java 线程池实现类名；执行时交给 `ExecutorHelper` 选择真实有界执行器。
    thread_pool: Option<String>,
}

impl BindWrapperCondition {
    /// 创建携带 Condition 级绑定数据的执行包装器。
    ///
    /// 参数 `inner` 是被修饰的执行对象，`bind_data` 是键值列表；执行时绑定数据
    /// 压入 Frame 并对内部对象可见。承接 Java `Condition#putBindData` 语义。
    #[must_use]
    pub fn new(inner: Arc<dyn Executable>, bind_data: Vec<(String, String)>) -> Self {
        Self {
            inner,
            bind_data,
            id: None,
            tag: None,
            thread_pool: None,
        }
    }

    /// 创建同时携带 Condition 公共属性的包装。
    ///
    /// 对应 Java Condition#setId、setTag、putBindData，以及 LoopCondition
    /// 的 setThreadPoolExecutorClass。Rust 用一个不可变包装保持对象安全。
    pub fn with_properties(
        inner: Arc<dyn Executable>,
        bind_data: Vec<(String, String)>,
        id: Option<String>,
        tag: Option<String>,
        thread_pool: Option<String>,
    ) -> Self {
        Self {
            inner,
            bind_data,
            id,
            tag,
            thread_pool,
        }
    }

    /// 返回保留的 Java 线程池实现类名。
    pub fn thread_pool(&self) -> Option<&str> {
        self.thread_pool.as_deref()
    }
}

#[async_trait]
impl Executable for BindWrapperCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let frame = frame
            .push_bind(&self.bind_data)
            .with_condition_thread_pool(self.thread_pool.as_deref());
        self.inner.execute(ctx, &frame).await
    }
    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or_else(|| self.inner.id())
    }
    fn tag(&self) -> Option<&str> {
        self.tag.as_deref().or_else(|| self.inner.tag())
    }
    fn is_pre_or_finally(&self) -> bool {
        self.inner.is_pre_or_finally()
    }
    async fn is_access(&self, ctx: &Ctx, frame: &Frame) -> bool {
        self.inner.is_access(ctx, frame).await
    }
}
