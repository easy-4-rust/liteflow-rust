//! Java `MethodWrapBean` 的 Rust 映射。

use serde_json::Value;

use crate::core::DeclComponent;
use crate::enums::{LiteFlowMethodEnum, NodeTypeEnum};
use crate::exception::{LFResult, LiteflowError};
use crate::slot::CmpContext;

use super::{LiteFlowMethodBean, ParameterWrapBean};

/// 保存一个声明式方法的注解、参数和重试元数据。
///
/// 对应 Java: `com.yomahub.liteflow.core.proxy.MethodWrapBean`。
#[derive(Debug, Clone)]
pub struct MethodWrapBean {
    method: LiteFlowMethodBean,
    liteflow_method: LiteFlowMethodEnum,
    node_type: NodeTypeEnum,
    liteflow_retry: Option<usize>,
    retry_for: Vec<String>,
    parameter_wrap_bean_list: Vec<ParameterWrapBean>,
}

impl MethodWrapBean {
    /// 创建声明式方法包装对象。
    ///
    /// 对应 Java: `MethodWrapBean#MethodWrapBean`。
    #[must_use]
    pub fn new(
        method: LiteFlowMethodBean,
        liteflow_method: LiteFlowMethodEnum,
        node_type: NodeTypeEnum,
        liteflow_retry: Option<usize>,
        retry_for: Vec<String>,
        parameter_wrap_bean_list: Vec<ParameterWrapBean>,
    ) -> Self {
        Self {
            method,
            liteflow_method,
            node_type,
            liteflow_retry,
            retry_for,
            parameter_wrap_bean_list,
        }
    }

    /// 返回 Rust 静态分派方法元数据。对应 Java: `MethodWrapBean#getMethod`。
    #[must_use]
    pub fn method(&self) -> &LiteFlowMethodBean {
        &self.method
    }

    /// 修改 Rust 静态分派方法元数据。对应 Java: `MethodWrapBean#setMethod`。
    pub fn set_method(&mut self, method: LiteFlowMethodBean) {
        self.method = method;
    }

    /// 返回 `@LiteflowMethod#value`。对应 Java: `MethodWrapBean#getLiteflowMethod`。
    #[must_use]
    pub fn liteflow_method(&self) -> LiteFlowMethodEnum {
        self.liteflow_method
    }

    /// 修改 `@LiteflowMethod#value`。对应 Java: `MethodWrapBean#setLiteflowMethod`。
    pub fn set_liteflow_method(&mut self, liteflow_method: LiteFlowMethodEnum) {
        self.liteflow_method = liteflow_method;
    }

    /// 返回声明式方法的节点类型。
    #[must_use]
    pub fn node_type(&self) -> NodeTypeEnum {
        self.node_type
    }

    /// 修改声明式方法的节点类型。
    pub fn set_node_type(&mut self, node_type: NodeTypeEnum) {
        self.node_type = node_type;
    }

    /// 返回 `@LiteflowRetry` 的重试次数。
    ///
    /// 对应 Java: `MethodWrapBean#getLiteflowRetry`。
    #[must_use]
    pub fn liteflow_retry(&self) -> Option<usize> {
        self.liteflow_retry
    }

    /// 修改 `@LiteflowRetry` 的重试次数。
    ///
    /// 对应 Java: `MethodWrapBean#setLiteflowRetry`。
    pub fn set_liteflow_retry(&mut self, liteflow_retry: Option<usize>) {
        self.liteflow_retry = liteflow_retry;
    }

    /// 返回可重试异常名称。
    #[must_use]
    pub fn retry_for(&self) -> &[String] {
        &self.retry_for
    }

    /// 修改可重试异常名称。
    pub fn set_retry_for(&mut self, retry_for: Vec<String>) {
        self.retry_for = retry_for;
    }

    /// 返回参数包装列表。对应 Java: `MethodWrapBean#getParameterWrapBeanList`。
    #[must_use]
    pub fn parameter_wrap_bean_list(&self) -> &[ParameterWrapBean] {
        &self.parameter_wrap_bean_list
    }

    /// 修改参数包装列表。对应 Java: `MethodWrapBean#setParameterWrapBeanList`。
    pub fn set_parameter_wrap_bean_list(
        &mut self,
        parameter_wrap_bean_list: Vec<ParameterWrapBean>,
    ) {
        self.parameter_wrap_bean_list = parameter_wrap_bean_list;
    }

    /// 校验事实参数并调用原始声明式组件。
    ///
    /// Java `DeclComponentProxy#loadMethodParameter` 在代理调用前逐个解析
    /// `@LiteflowFact`；Rust 先验证 Bean 名称存在，再由过程宏生成的强类型分派完成
    /// downcast。对应 Java: `DeclComponentProxy.AopInvocationHandler#invoke`。
    pub async fn invoke(
        &self,
        raw_bean: &dyn DeclComponent,
        context: &CmpContext,
    ) -> LFResult<Value> {
        for parameter in &self.parameter_wrap_bean_list {
            if let Some(fact) = parameter.fact() {
                if !context.inner.beans.contains_key(fact) {
                    return Err(LiteflowError::ParameterFact(format!(
                        "decl method[{}] parameter[{}:{}] fact bean[{}] not found",
                        self.method.method_name(),
                        parameter.index(),
                        parameter.parameter_type(),
                        fact
                    )));
                }
            }
        }
        raw_bean.call(self.method.method_name(), context).await
    }

    /// 判断错误是否命中声明式方法的重试范围。
    #[must_use]
    pub fn is_retry_for(&self, error: &LiteflowError) -> bool {
        if self.retry_for.is_empty() {
            return self.liteflow_retry.unwrap_or_default() > 0;
        }
        let kind = match error {
            LiteflowError::NodeExec { kind, .. } => kind.as_str(),
            _ => std::any::type_name_of_val(error),
        };
        self.retry_for.iter().any(|candidate| {
            let simple = candidate.rsplit('.').next().unwrap_or(candidate);
            kind.ends_with(simple)
                || kind.trim_end_matches("Exception") == simple.trim_end_matches("Exception")
        })
    }
}
