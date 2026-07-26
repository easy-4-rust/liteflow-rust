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

    fn name(&self) -> &str {
        self.decl.method_name(&self.method).unwrap_or("")
    }

    fn node_type(&self) -> Option<crate::enums::NodeTypeEnum> {
        self.decl.method_node_type(&self.method)
    }

    fn retry_count(&self) -> usize {
        self.decl.method_retry_count(&self.method)
    }

    fn is_retry_for(&self, error: &LiteflowError) -> bool {
        self.decl.is_method_retry_for(&self.method, error)
    }
}
