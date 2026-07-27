//! 对应 Spring Boot Web 入口的 Axum 0.8 适配对象。

use std::sync::Arc;

use axum::{Json, Router, extract::Path, http::StatusCode, routing::post};
use vernal_axum::{VernalComponent, VernalRouterExt};
use vernal_context::ApplicationContext;

use crate::{LiteflowExecuteRequest, LiteflowExecuteResponse, LiteflowRuntime};

/// 提供 LiteFlow HTTP 路由并接入 Vernal 请求作用域。
pub struct LiteflowAxumRouter;

impl LiteflowAxumRouter {
    /// 创建尚未附加应用上下文的路由。
    #[must_use]
    pub fn router() -> Router {
        Router::new().route("/liteflow/execute/{chain_id}", post(execute))
    }

    /// 创建并附加 Vernal Context；请求中从同一容器解析 LiteflowRuntime。
    #[must_use]
    pub fn with_context(context: Arc<ApplicationContext>) -> Router {
        Self::router().with_vernal(context)
    }
}

async fn execute(
    Path(chain_id): Path<String>,
    VernalComponent(runtime): VernalComponent<LiteflowRuntime>,
    Json(request): Json<LiteflowExecuteRequest>,
) -> (StatusCode, Json<LiteflowExecuteResponse>) {
    let result = match request.request_id {
        Some(request_id) => {
            runtime
                .try_execute_with_rid(&chain_id, request.data, request_id)
                .await
        }
        None => runtime.try_execute(&chain_id, request.data).await,
    };
    let initialization_failed = result.is_err();
    let response = result.unwrap_or_else(|error| {
        liteflow_core::LiteflowResponse::initialization_failure(
            "rule-init-failed",
            &chain_id,
            serde_json::Value::Null,
            error.to_string(),
        )
    });
    let status = if response.is_success() {
        StatusCode::OK
    } else if initialization_failed {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    (status, Json(LiteflowExecuteResponse::from(&response)))
}
