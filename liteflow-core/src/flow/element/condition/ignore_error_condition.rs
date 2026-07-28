//! 对应 Condition 基类的 ignoreError 语义（非 WHEN 场景）：
//! 包裹一层，吞掉子条件异常并记入 slot。

use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 为普通 Condition 实现 ignoreError 修饰语义的 Rust 包装对象。
///
/// Java 将该标志保存在 Condition 基类并由执行主干读取；Rust 使用独立包装器
/// 隔离同一个 Condition 在不同 EL 出现位置的修饰状态，不对应独立 Java 类。
pub struct IgnoreErrorCondition {
    inner: Arc<dyn Executable>,
}

impl IgnoreErrorCondition {
    /// 创建忽略普通执行错误的 Condition 包装器。
    ///
    /// 参数 `inner` 是被包装的真实可执行对象；ChainEnd 仍会向上传播。
    #[must_use]
    pub fn new(inner: Arc<dyn Executable>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Executable for IgnoreErrorCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        match self.inner.execute(ctx, frame).await {
            Ok(v) => Ok(v),
            Err(LiteflowError::ChainEnd(message)) => Err(LiteflowError::ChainEnd(message)),
            Err(e) => {
                ctx.set_exception(&e.to_string());
                Ok(Value::Null)
            }
        }
    }

    fn id(&self) -> &str {
        "IGNORE_ERROR"
    }
}
