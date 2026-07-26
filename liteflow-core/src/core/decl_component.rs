//! 对应 core.proxy.DeclComponentProxy + annotation.LiteflowMethod：
//! 声明式组件——一个组件暴露多个具名方法，EL 中以 `cmpId.methodName` 引用。

use crate::exception::LiteflowError;
use crate::slot::CmpContext;
use async_trait::async_trait;
use serde_json::Value;

/// 声明式组件（对应 @LiteflowCmpDefine 类 + @LiteflowMethod 方法）
#[async_trait]
pub trait DeclComponent: Send + Sync + 'static {
    /// 按方法名调用（对应 LiteFlowMethodEnum 分派）
    async fn call(&self, method: &str, ctx: &CmpContext) -> Result<Value, LiteflowError>;
}
