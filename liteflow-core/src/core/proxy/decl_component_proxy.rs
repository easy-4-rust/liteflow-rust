//! Java `DeclComponentProxy` 的 Rust 映射。

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::core::DeclComponent;
use crate::enums::{LiteFlowMethodEnum, NodeTypeEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::slot::CmpContext;

use super::{DeclWarpBean, MethodWrapBean};

/// 声明式组件代理核心。
///
/// Java 使用 ByteBuddy 动态生成 `NodeComponent` 子类；Rust 通过本对象持有编译期
/// 生成的 `DeclComponent` 静态分派表，同时保留节点类型一致性、方法查找、事实
/// 参数验证及重试元数据传播。
///
/// 对应 Java: `com.yomahub.liteflow.core.proxy.DeclComponentProxy`。
#[derive(Clone)]
pub struct DeclComponentProxy {
    decl_warp_bean: Arc<DeclWarpBean>,
}

impl DeclComponentProxy {
    /// 创建声明式组件代理。对应 Java: `DeclComponentProxy#DeclComponentProxy`。
    #[must_use]
    pub fn new(decl_warp_bean: DeclWarpBean) -> Self {
        Self {
            decl_warp_bean: Arc::new(decl_warp_bean),
        }
    }

    /// 校验元数据并生成声明式组件代理。
    ///
    /// 同一 nodeId 的方法必须映射为同一 NodeType；方法名也必须唯一。对应 Java:
    /// `DeclComponentProxy#getProxy`。
    pub fn get_proxy(self) -> LFResult<Arc<dyn DeclComponent>> {
        let methods = self.decl_warp_bean.method_wrap_bean_list();
        if methods.is_empty() {
            return Err(LiteflowError::ComponentProxyError(format!(
                "decl component[{}:{}] has no LiteflowMethod",
                self.decl_warp_bean.node_id(),
                self.decl_warp_bean.raw_clazz()
            )));
        }

        // Java 按映射的 NodeComponent class 去重；Rust 对等检查 NodeTypeEnum。
        let node_types = methods
            .iter()
            .map(MethodWrapBean::node_type)
            .collect::<HashSet<_>>();
        if node_types.len() != 1 || !node_types.contains(&self.decl_warp_bean.node_type()) {
            return Err(LiteflowError::ComponentProxyError(format!(
                "the node type of the same nodeId must be identical: nodeId[{}], type[{}]",
                self.decl_warp_bean.node_id(),
                self.decl_warp_bean.node_type().get_code()
            )));
        }

        let mut method_names = HashSet::new();
        for method in methods {
            if !method_names.insert(method.method().method_name()) {
                return Err(LiteflowError::ComponentProxyError(format!(
                    "duplicate LiteflowMethod[{}] in decl component[{}]",
                    method.method().method_name(),
                    self.decl_warp_bean.node_id()
                )));
            }
        }
        Ok(Arc::new(self))
    }

    fn method_wrap_bean(&self, method: &str) -> Option<&MethodWrapBean> {
        self.decl_warp_bean
            .method_wrap_bean_list()
            .iter()
            .find(|candidate| candidate.method().method_name() == method)
    }

    /// 调用声明式组件中与 LiteFlow 方法名匹配的真实业务方法。
    ///
    /// 参数 `method` 对应 Java 动态代理收到的 `Method#getName`，`context` 提供
    /// NodeComponent 自身参数、普通参数和 `@LiteflowFact` 上下文事实。不存在方法
    /// 时返回代理错误；业务方法错误保持原始 `LiteflowError`。
    ///
    /// Rust 不生成 ByteBuddy `InvocationHandler`，该方法直接承担 Java 内部
    /// `AopInvocationHandler#invoke` 的查找、参数装载与调用职责。
    /// 对应 Java: `DeclComponentProxy.AopInvocationHandler#invoke`。
    pub async fn invoke(&self, method: &str, context: &CmpContext) -> LFResult<Value> {
        let method_wrap_bean = self.method_wrap_bean(method).ok_or_else(|| {
            LiteflowError::Proxy(format!(
                "decl component[{}] has no LiteflowMethod[{method}]",
                self.decl_warp_bean.node_id()
            ))
        })?;
        method_wrap_bean
            .invoke(self.decl_warp_bean.raw_bean().as_ref(), context)
            .await
    }

    /// 调用带真实执行错误参数的声明式生命周期方法。
    ///
    /// 对应 Java: `DeclComponentProxy.AopInvocationHandler#invoke` 的
    /// `onError(Exception)` 分支。
    pub async fn invoke_with_error(
        &self,
        method: &str,
        context: &CmpContext,
        error: &LiteflowError,
    ) -> LFResult<Value> {
        let method_wrap_bean = self.method_wrap_bean(method).ok_or_else(|| {
            LiteflowError::Proxy(format!(
                "decl component[{}] has no LiteflowMethod[{method}]",
                self.decl_warp_bean.node_id()
            ))
        })?;
        method_wrap_bean
            .invoke_with_error(self.decl_warp_bean.raw_bean().as_ref(), context, error)
            .await
    }
}

#[async_trait]
impl DeclComponent for DeclComponentProxy {
    async fn call(&self, method: &str, context: &CmpContext) -> LFResult<Value> {
        self.invoke(method, context).await
    }

    async fn call_with_error(
        &self,
        method: &str,
        context: &CmpContext,
        error: &LiteflowError,
    ) -> LFResult<Value> {
        self.invoke_with_error(method, context, error).await
    }

    fn has_method(&self, method: &str) -> bool {
        self.method_wrap_bean(method).is_some()
    }

    fn method_node_type(&self, method: &str) -> Option<NodeTypeEnum> {
        self.method_wrap_bean(method).map(MethodWrapBean::node_type)
    }

    fn method_name(&self, method: &str) -> Option<&str> {
        self.method_wrap_bean(method)
            .map(|_| self.decl_warp_bean.node_name())
            .filter(|name| !name.is_empty())
    }

    fn method_retry_count(&self, method: &str) -> usize {
        self.method_wrap_bean(method)
            .and_then(MethodWrapBean::liteflow_retry)
            .unwrap_or_default()
    }

    fn is_method_retry_for(&self, method: &str, error: &LiteflowError) -> bool {
        self.method_wrap_bean(method)
            .is_some_and(|metadata| metadata.is_retry_for(error))
    }

    fn method_for_lifecycle(&self, liteflow_method: LiteFlowMethodEnum) -> Option<&str> {
        self.decl_warp_bean
            .method_wrap_bean_list()
            .iter()
            .find(|candidate| candidate.liteflow_method() == liteflow_method)
            .map(|candidate| candidate.method().method_name())
    }
}
