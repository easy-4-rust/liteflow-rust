//! Web 执行入口的响应对象。

use liteflow_core::LiteflowResponse;
use serde::{Deserialize, Serialize};

/// 可序列化、不会暴露 Slot 内部对象的 LiteFlow HTTP 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteflowExecuteResponse {
    /// 请求 id。
    pub request_id: String,
    /// 链路 id。
    pub chain_id: String,
    /// 是否成功。
    pub success: bool,
    /// 稳定执行消息。
    pub message: String,
    /// 可选失败原因。
    pub cause: Option<String>,
    /// 执行步骤字符串。
    pub steps: String,
}

impl From<&LiteflowResponse> for LiteflowExecuteResponse {
    fn from(response: &LiteflowResponse) -> Self {
        Self {
            request_id: response.request_id.clone(),
            chain_id: response.chain_id.clone(),
            success: response.success,
            message: response.message.clone(),
            cause: response.cause.clone(),
            steps: response.step_str(),
        }
    }
}
