//! 对应 core.proxy.DeclComponentProxy + annotation.LiteflowMethod：
//! 声明式组件——一个组件暴露多个具名方法，EL 中以 `cmpId.methodName` 引用。

use crate::exception::LiteflowError;
use crate::slot::CmpContext;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 声明式组件（对应 @LiteflowCmpDefine 类 + @LiteflowMethod 方法）
#[async_trait]
pub trait DeclComponent: Send + Sync + 'static {
    /// 按方法名调用（对应 LiteFlowMethodEnum 分派）
    async fn call(&self, method: &str, ctx: &CmpContext) -> Result<Value, LiteflowError>;
}

/// 把 `cmpId.methodName` 包装成普通 NodeComponent（对应 DeclComponentProxy 的方法代理）
pub struct DeclMethodComponent {
    decl: Arc<dyn DeclComponent>,
    method: String,
}

impl DeclMethodComponent {
    pub fn new(decl: Arc<dyn DeclComponent>, method: impl Into<String>) -> Self {
        Self {
            decl,
            method: method.into(),
        }
    }
}

#[async_trait]
impl crate::core::node_component::NodeComponent for DeclMethodComponent {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.decl.call(&self.method, ctx).await
    }
}
