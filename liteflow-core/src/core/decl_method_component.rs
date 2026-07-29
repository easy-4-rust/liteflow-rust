//! 对应 Java: `com.yomahub.liteflow.core.proxy.DeclComponentProxy`。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::enums::LiteFlowMethodEnum;
use crate::exception::LiteflowError;
use crate::slot::CmpContext;

use super::{DeclComponent, NodeComponent};

/// 把 `cmpId.methodName` 包装成普通节点组件。
pub struct DeclMethodComponent {
    decl: Arc<dyn DeclComponent>,
    method: String,
    lifecycle_node: bool,
}

impl DeclMethodComponent {
    /// 创建声明式方法代理。
    #[must_use]
    pub fn new(decl: Arc<dyn DeclComponent>, method: impl Into<String>) -> Self {
        Self {
            decl,
            method: method.into(),
            lifecycle_node: false,
        }
    }

    /// 从声明式节点的主方法构造完整生命周期代理。
    ///
    /// 返回 `None` 表示该 `nodeId` 没有 Java 要求的 process 主方法。对应 Java:
    /// `SpringDeclComponentParser#parseDeclBean` 与 `DeclComponentProxy#getProxy`。
    #[must_use]
    pub fn for_node(decl: Arc<dyn DeclComponent>) -> Option<Self> {
        let method = [
            LiteFlowMethodEnum::Process,
            LiteFlowMethodEnum::ProcessSwitch,
            LiteFlowMethodEnum::ProcessBoolean,
            LiteFlowMethodEnum::ProcessFor,
            LiteFlowMethodEnum::ProcessIterator,
        ]
        .into_iter()
        .find_map(|role| decl.method_for_lifecycle(role).map(ToOwned::to_owned))?;
        Some(Self {
            decl,
            method,
            lifecycle_node: true,
        })
    }

    async fn call_lifecycle(
        &self,
        liteflow_method: LiteFlowMethodEnum,
        ctx: &CmpContext,
    ) -> Result<Option<Value>, LiteflowError> {
        let Some(method) = self.decl.method_for_lifecycle(liteflow_method) else {
            return Ok(None);
        };
        self.decl.call(method, ctx).await.map(Some)
    }

    async fn call_error_lifecycle(
        &self,
        ctx: &CmpContext,
        error: &LiteflowError,
    ) -> Result<Option<Value>, LiteflowError> {
        let Some(method) = self.decl.method_for_lifecycle(LiteFlowMethodEnum::OnError) else {
            return Ok(None);
        };
        self.decl
            .call_with_error(method, ctx, error)
            .await
            .map(Some)
    }
}

#[async_trait]
impl NodeComponent for DeclMethodComponent {
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError> {
        self.decl.call(&self.method, ctx).await
    }

    async fn before_process(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        if self.lifecycle_node {
            self.call_lifecycle(LiteFlowMethodEnum::BeforeProcess, ctx)
                .await?;
        }
        Ok(())
    }

    async fn on_success(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        if self.lifecycle_node {
            self.call_lifecycle(LiteFlowMethodEnum::OnSuccess, ctx)
                .await?;
        }
        Ok(())
    }

    async fn after_process(&self, ctx: &CmpContext) {
        if self.lifecycle_node {
            let _ = self
                .call_lifecycle(LiteFlowMethodEnum::AfterProcess, ctx)
                .await;
        }
    }

    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        if self.lifecycle_node {
            let _ = self.call_error_lifecycle(ctx, error).await;
        }
    }

    async fn is_access_async(&self, ctx: &CmpContext) -> Result<bool, LiteflowError> {
        if !self.lifecycle_node {
            return Ok(true);
        }
        match self.call_lifecycle(LiteFlowMethodEnum::IsAccess, ctx).await {
            Ok(Some(value)) => value.as_bool().ok_or_else(|| {
                LiteflowError::CmpDefine(format!(
                    "decl method[isAccess] must return boolean, got {value}"
                ))
            }),
            Ok(None) => Ok(true),
            Err(error) => Err(error),
        }
    }

    async fn is_continue_on_error_async(&self, ctx: &CmpContext) -> Result<bool, LiteflowError> {
        if !self.lifecycle_node {
            return Ok(false);
        }
        match self
            .call_lifecycle(LiteFlowMethodEnum::IsContinueOnError, ctx)
            .await?
        {
            Some(value) => value.as_bool().ok_or_else(|| {
                LiteflowError::CmpDefine(format!(
                    "decl method[isContinueOnError] must return boolean, got {value}"
                ))
            }),
            None => Ok(false),
        }
    }

    async fn is_end_async(&self, ctx: &CmpContext) -> Result<bool, LiteflowError> {
        if self.lifecycle_node {
            match self.call_lifecycle(LiteFlowMethodEnum::IsEnd, ctx).await? {
                Some(value) => {
                    return value.as_bool().ok_or_else(|| {
                        LiteflowError::CmpDefine(format!(
                            "decl method[isEnd] must return boolean, got {value}"
                        ))
                    });
                }
                None => {}
            }
        }
        Ok(ctx.inner.ended.load(std::sync::atomic::Ordering::Acquire))
    }

    fn is_rollback(&self) -> bool {
        self.lifecycle_node
            && self
                .decl
                .method_for_lifecycle(LiteFlowMethodEnum::Rollback)
                .is_some()
    }

    async fn rollback(&self, ctx: &CmpContext) -> Result<(), LiteflowError> {
        if self.lifecycle_node {
            self.call_lifecycle(LiteFlowMethodEnum::Rollback, ctx)
                .await?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        self.decl.method_name(&self.method).unwrap_or("")
    }

    async fn display_name_async(&self, ctx: &CmpContext) -> Result<Option<String>, LiteflowError> {
        if !self.lifecycle_node {
            return Ok(None);
        }
        match self
            .call_lifecycle(LiteFlowMethodEnum::GetDisplayName, ctx)
            .await?
        {
            Some(Value::String(name)) => Ok(Some(name)),
            Some(value) => Err(LiteflowError::CmpDefine(format!(
                "decl method[getDisplayName] must return string, got {value}"
            ))),
            None => Ok(None),
        }
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

    async fn node_executor_class_async(
        &self,
        ctx: &CmpContext,
    ) -> Result<Option<String>, LiteflowError> {
        if !self.lifecycle_node {
            return Ok(None);
        }
        match self
            .call_lifecycle(LiteFlowMethodEnum::GetNodeExecutorClass, ctx)
            .await?
        {
            Some(Value::String(class_name)) => Ok(Some(class_name)),
            Some(value) => Err(LiteflowError::CmpDefine(format!(
                "decl method[getNodeExecutorClass] must return string, got {value}"
            ))),
            None => Ok(None),
        }
    }
}
