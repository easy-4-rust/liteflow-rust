use super::RequestIdGenerator;

/// 默认请求ID生成器
///
/// 用于生成默认的请求ID
#[derive(Default)]
pub struct DefaultRequestIdGenerator;

impl RequestIdGenerator for DefaultRequestIdGenerator {
    /// 生成默认的请求ID
    ///
    /// 返回默认的请求ID
    fn generate_request_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
