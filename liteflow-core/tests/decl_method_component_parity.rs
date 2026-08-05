//! `DeclMethodComponent` 生命周期错误路径与短路补测。
//!
//! 对应 Java: `DeclComponentProxy.AopInvocationHandler` 分派的
//! isAccess/isEnd/isContinueOnError/getDisplayName/getNodeExecutorClass 与
//! `MethodWrapBean#invokeWithError` 的事实缺失校验。现有 s6_proxy 已覆盖
//! 正常生命周期；本文件补足非法返回类型与未注册方法路径。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::Slot;
use liteflow_core::core::proxy::{DeclWarpBean, MethodWrapBean, ParameterWrapBean};
use liteflow_core::core::{DeclComponent, DeclMethodComponent, NodeComponent};
use liteflow_core::enums::{LiteFlowMethodEnum, NodeTypeEnum};
use liteflow_core::exception::LiteflowError;
use liteflow_core::slot::CmpContext;
use serde_json::{Value, json};

/// 按生命周期角色返回可配置值的声明式组件。
struct ConfigurableDecl {
    /// 生命周期角色 -> 方法名（LiteFlowMethodEnum 无 Hash，用 Vec 顺序查找）
    lifecycle: Vec<(LiteFlowMethodEnum, String)>,
    /// 方法名 -> 返回载荷
    returns: HashMap<String, Value>,
    /// 方法名 -> 是否报错
    errors: HashMap<String, String>,
}

#[async_trait]
impl DeclComponent for ConfigurableDecl {
    async fn call(&self, method: &str, _context: &CmpContext) -> Result<Value, LiteflowError> {
        if let Some(message) = self.errors.get(method) {
            return Err(LiteflowError::Custom(message.clone()));
        }
        self.returns
            .get(method)
            .cloned()
            .ok_or_else(|| LiteflowError::CmpDefine(format!("no stub for {method}")))
    }

    async fn call_with_error(
        &self,
        method: &str,
        _context: &CmpContext,
        _error: &LiteflowError,
    ) -> Result<Value, LiteflowError> {
        self.call(method, _context).await
    }

    fn has_method(&self, method: &str) -> bool {
        self.returns.contains_key(method)
    }

    fn method_node_type(&self, method: &str) -> Option<NodeTypeEnum> {
        self.has_method(method).then_some(NodeTypeEnum::Common)
    }

    fn method_name(&self, method: &str) -> Option<&str> {
        self.has_method(method).then_some("可配置组件")
    }

    fn method_retry_count(&self, method: &str) -> usize {
        if self.has_method(method) {
            2
        } else {
            Default::default()
        }
    }

    fn is_method_retry_for(&self, _method: &str, _error: &LiteflowError) -> bool {
        false
    }

    fn method_for_lifecycle(&self, liteflow_method: LiteFlowMethodEnum) -> Option<&str> {
        self.lifecycle
            .iter()
            .find(|(role, _)| *role == liteflow_method)
            .map(|(_, method)| method.as_str())
    }
}

fn lifecycle_decl(
    lifecycle: Vec<(LiteFlowMethodEnum, String)>,
    returns: HashMap<String, Value>,
) -> Arc<dyn DeclComponent> {
    Arc::new(ConfigurableDecl {
        lifecycle,
        returns,
        errors: HashMap::new(),
    })
}

fn ctx() -> CmpContext {
    let slot = Arc::new(Slot::new("RID-DECL".to_string(), "decl_chain", Value::Null));
    CmpContext {
        inner: slot,
        node: liteflow_core::NodeRef::new("decl-node"),
        frame: liteflow_core::Frame::root(),
    }
}

/// for_node 在没有任何主方法时返回 None，与 Java 声明的 process 主方法缺失一致。
#[test]
fn for_node_returns_none_without_main_method() {
    let decl = lifecycle_decl(Vec::new(), HashMap::new());
    assert!(DeclMethodComponent::for_node(decl).is_none());
}

/// isAccess 返回非布尔值时按 Java 声明组件约束报错。
#[tokio::test]
async fn is_access_rejects_non_boolean_return() {
    let decl = lifecycle_decl(
        vec![
            (LiteFlowMethodEnum::Process, "process".to_string()),
            (LiteFlowMethodEnum::IsAccess, "checkAccess".to_string()),
        ],
        HashMap::from([
            ("process".to_string(), Value::Null),
            ("checkAccess".to_string(), json!("yes")),
        ]),
    );
    let component = DeclMethodComponent::for_node(decl).expect("主方法应存在");
    let error = component
        .is_access_async(&ctx())
        .await
        .expect_err("非布尔应报错");
    assert!(error.to_string().contains("isAccess"));
    assert!(error.to_string().contains("must return boolean"));
}

/// isContinueOnError 返回非布尔值时报错。
#[tokio::test]
async fn is_continue_on_error_rejects_non_boolean_return() {
    let decl = lifecycle_decl(
        vec![
            (LiteFlowMethodEnum::Process, "process".to_string()),
            (
                LiteFlowMethodEnum::IsContinueOnError,
                "continueCheck".to_string(),
            ),
        ],
        HashMap::from([
            ("process".to_string(), Value::Null),
            ("continueCheck".to_string(), json!(7)),
        ]),
    );
    let component = DeclMethodComponent::for_node(decl).expect("主方法应存在");
    let error = component
        .is_continue_on_error_async(&ctx())
        .await
        .expect_err("非布尔应报错");
    assert!(error.to_string().contains("isContinueOnError"));
}

/// getDisplayName 返回非字符串值时报错。
#[tokio::test]
async fn display_name_rejects_non_string_return() {
    let decl = lifecycle_decl(
        vec![
            (LiteFlowMethodEnum::Process, "process".to_string()),
            (
                LiteFlowMethodEnum::GetDisplayName,
                "displayName".to_string(),
            ),
        ],
        HashMap::from([
            ("process".to_string(), Value::Null),
            ("displayName".to_string(), json!(42)),
        ]),
    );
    let component = DeclMethodComponent::for_node(decl).expect("主方法应存在");
    let error = component
        .display_name_async(&ctx())
        .await
        .expect_err("非字符串应报错");
    assert!(error.to_string().contains("getDisplayName"));
}

/// getNodeExecutorClass 返回非字符串值时报错。
#[tokio::test]
async fn node_executor_class_rejects_non_string_return() {
    let decl = lifecycle_decl(
        vec![
            (LiteFlowMethodEnum::Process, "process".to_string()),
            (
                LiteFlowMethodEnum::GetNodeExecutorClass,
                "executorClass".to_string(),
            ),
        ],
        HashMap::from([
            ("process".to_string(), Value::Null),
            ("executorClass".to_string(), json!(false)),
        ]),
    );
    let component = DeclMethodComponent::for_node(decl).expect("主方法应存在");
    let error = component
        .node_executor_class_async(&ctx())
        .await
        .expect_err("非字符串应报错");
    assert!(error.to_string().contains("getNodeExecutorClass"));
}

/// 普通代理（非生命周期节点）跳过所有生命周期调用。
#[tokio::test]
async fn plain_proxy_skips_lifecycle_hooks() {
    let decl = lifecycle_decl(Vec::new(), HashMap::new());
    let component = DeclMethodComponent::new(decl, "process");
    // 未注册方法调用返回代理错误
    let error = component.process(&ctx()).await.expect_err("未注册应报错");
    assert!(error.to_string().contains("no stub for process"));

    // 生命周期钩子直接短路，不触碰 decl
    assert!(component.before_process(&ctx()).await.is_ok());
    assert!(component.on_success(&ctx()).await.is_ok());
    component.after_process(&ctx()).await;
    component
        .on_error(&ctx(), &LiteflowError::Custom("x".into()))
        .await;
    assert!(!component.is_rollback());
    assert!(component.rollback(&ctx()).await.is_ok());
}

/// MethodWrapBean#invokeWithError 在事实 Bean 缺失时先于方法调用报错。
#[tokio::test]
async fn invoke_with_error_checks_fact_before_dispatch() {
    let method = MethodWrapBean::new(
        liteflow_core::core::proxy::LiteFlowMethodBean::new(
            "handleError",
            LiteFlowMethodEnum::OnError,
        ),
        LiteFlowMethodEnum::OnError,
        NodeTypeEnum::Common,
        None,
        Vec::new(),
        vec![ParameterWrapBean::new(
            "Arc<MissingFact>",
            Some("missingFact"),
            0,
        )],
    );
    let declaration = DeclWarpBean::new(
        "decl",
        "声明式",
        NodeTypeEnum::Common,
        lifecycle_decl(Vec::new(), HashMap::new()),
        "tests::ConfigurableDecl",
        vec![method.clone()],
    );

    let error = method
        .invoke_with_error(
            declaration.raw_bean().as_ref(),
            &ctx(),
            &LiteflowError::Custom("boom".into()),
        )
        .await
        .expect_err("缺失事实 Bean 应报错");
    assert!(
        error
            .to_string()
            .contains("fact bean[missingFact] not found")
    );
}
