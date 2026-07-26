//! 对应 Quarkus 扩展职责的 Actix Web 4 适配对象。

use std::sync::Arc;

use actix_web::{HttpResponse, Responder, web};
use vernal_actix_web::{VernalActixComponent, VernalActixMiddleware};
use vernal_context::ApplicationContext;

use crate::{LiteflowExecuteRequest, LiteflowExecuteResponse, LiteflowRuntime};

/// 向 Actix App 注册 LiteFlow 资源与 Vernal 请求作用域。
pub struct LiteflowActixService;

impl LiteflowActixService {
    /// 配置 `/liteflow/execute/{chain_id}`。
    pub fn configure(config: &mut web::ServiceConfig, context: Arc<ApplicationContext>) {
        config.service(
            web::resource("/liteflow/execute/{chain_id}")
                .wrap(VernalActixMiddleware::new(context))
                .route(web::post().to(execute)),
        );
    }
}

async fn execute(
    chain_id: web::Path<String>,
    runtime: VernalActixComponent<LiteflowRuntime>,
    request: web::Json<LiteflowExecuteRequest>,
) -> impl Responder {
    let request = request.into_inner();
    let response = match request.request_id {
        Some(request_id) => {
            runtime
                .execute_with_rid(&chain_id, request.data, request_id)
                .await
        }
        None => runtime.execute(&chain_id, request.data).await,
    };
    let body = LiteflowExecuteResponse::from(&response);
    if response.is_success() {
        HttpResponse::Ok().json(body)
    } else {
        HttpResponse::UnprocessableEntity().json(body)
    }
}
