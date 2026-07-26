//! Web 执行入口的请求对象。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 执行 LiteFlow 链路的 HTTP 请求。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiteflowExecuteRequest {
    /// 传给链路的请求数据。
    pub data: Value,
    /// 可选显式 request id。
    pub request_id: Option<String>,
}
