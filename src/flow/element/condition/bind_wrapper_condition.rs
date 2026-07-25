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
}

impl BindWrapperCondition {
    pub fn new(inner: Arc<dyn Executable>, bind_data: Vec<(String, String)>) -> Self {
        Self { inner, bind_data }
    }
}

#[async_trait]
impl Executable for BindWrapperCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let frame = frame.push_bind(&self.bind_data);
        self.inner.execute(ctx, &frame).await
    }
    fn id(&self) -> &str {
        self.inner.id()
    }
    fn tag(&self) -> Option<&str> {
        self.inner.tag()
    }
    fn is_pre_or_finally(&self) -> bool {
        self.inner.is_pre_or_finally()
    }
    async fn is_access(&self, ctx: &Ctx, frame: &Frame) -> bool {
        self.inner.is_access(ctx, frame).await
    }
}
