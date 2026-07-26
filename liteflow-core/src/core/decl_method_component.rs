//! 对应 Java: `com.yomahub.liteflow.core.proxy.DeclComponentProxy`。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::exception::LiteflowError;
use crate::slot::CmpContext;

use super::{DeclComponent, NodeComponent};

/// 把 `cmpId.methodName` 包装成普通节点组件。
pub struct DeclMethodComponent {
    decl: Arc<dyn DeclComponent>,
    method: String,
}

impl DeclMethodComponent {
    /// 创建声明式方法代理。
    #[must_use]
    pub fn new(decl: Arc<dyn DeclComponent>, method: impl Into<String>) -> Self {
        Self {
            decl,
            method: method.into(),
        }
    }
}

#[async_trait]
impl NodeComponent for DeclMethodComponent {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.decl.call(&self.method, ctx).await
    }
}
