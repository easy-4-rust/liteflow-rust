// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/providers/bedrock.rs。
// 修改：
// - import 路径调整为 liteflow-agent-provider-core；
// - 精简：去掉 EC2 IMDS 凭证获取、图片/缓存点内容块，保留 SigV4 签名 + 文本/工具 Converse chat；
// - 凭证改为构造时显式传入（而非仅环境变量）。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! AWS Bedrock provider with manual SigV4 signing.
//!
//! 通过 AWS SigV4 签名调用 Bedrock Converse API，可接入 Claude（Bedrock 版）、
//! Titan、Llama 等模型。

use super::AwsCredentials;
use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use liteflow_agent_provider_core::{
    ChatMessage, ChatRequest as ProviderChatRequest, ChatResponse as ProviderChatResponse,
    Provider, TokenUsage, ToolCall as ProviderToolCall, ToolSpec,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const SIGNING_SERVICE: &str = "bedrock";
const ENDPOINT_PREFIX: &str = "bedrock-runtime";
const DEFAULT_MAX_TOKENS: u32 = 4096;

// ── AWS SigV4 Signing ──

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac =
        <HmacSha256 as KeyInit>::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// 构建 SigV4 Authorization 头。headers 必须按小写名排序。
fn build_authorization_header(
    credentials: &AwsCredentials,
    method: &str,
    canonical_uri: &str,
    query_string: &str,
    headers: &[(String, String)],
    payload: &[u8],
    timestamp: &chrono::DateTime<chrono::Utc>,
) -> String {
    let date_stamp = timestamp.format("%Y%m%d").to_string();
    let amz_date = timestamp.format("%Y%m%dT%H%M%SZ").to_string();

    let mut canonical_headers = String::new();
    for (k, v) in headers {
        canonical_headers.push_str(k);
        canonical_headers.push(':');
        canonical_headers.push_str(v);
        canonical_headers.push('\n');
    }

    let signed_headers: String = headers
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let payload_hash = sha256_hex(payload);
    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{query_string}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );

    let credential_scope = format!(
        "{date_stamp}/{}/{SIGNING_SERVICE}/aws4_request",
        credentials.region
    );

    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    let signing_key = derive_signing_key(
        &credentials.secret_access_key,
        &date_stamp,
        &credentials.region,
        SIGNING_SERVICE,
    );

    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    format!(
        "AWS4-HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}",
        credentials.access_key_id
    )
}

// ── Converse API Types ──

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConverseRequest {
    messages: Vec<ConverseMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<TextBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inference_config: Option<InferenceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_config: Option<ToolConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConverseMessage {
    role: String,
    content: Vec<BedrockContentBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum BedrockContentBlock {
    Text(TextBlock),
    ToolUse(ToolUseWrapper),
    ToolResult(ToolResultWrapper),
}

#[derive(Debug, Serialize, Deserialize)]
struct TextBlock {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolUseWrapper {
    tool_use: ToolUseBlock,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolUseBlock {
    tool_use_id: String,
    name: String,
    input: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolResultWrapper {
    tool_result: ToolResultBlock,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolResultBlock {
    tool_use_id: String,
    #[serde(rename = "content")]
    content: Vec<TextBlock>,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InferenceConfig {
    max_tokens: u32,
    temperature: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolConfig {
    tools: Vec<BedrockTool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BedrockTool {
    tool_spec: BedrockToolSpec,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BedrockToolSpec {
    name: String,
    description: String,
    input_schema: InputSchema,
}

#[derive(Debug, Serialize)]
struct InputSchema {
    json: serde_json::Value,
}

// ── Response Types ──

#[derive(Debug, Deserialize)]
struct ConverseResponse {
    output: Option<ConverseOutput>,
    #[serde(default)]
    usage: Option<BedrockUsage>,
    #[serde(default)]
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConverseOutput {
    message: Option<ConverseOutputMessage>,
}

#[derive(Debug, Deserialize)]
struct ConverseOutputMessage {
    #[serde(default)]
    content: Vec<BedrockContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BedrockUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

// ── Provider ──

/// 通过 AWS SigV4 调用 Bedrock Converse API 的模型提供商。
///
/// 对应 Java: 无（Rust 提供商基础设施；源自 ZeroClaw `BedrockProvider`）。
pub struct BedrockProvider {
    credentials: AwsCredentials,
    max_tokens: u32,
    client: Client,
}

impl BedrockProvider {
    /// 从环境变量创建（AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY/AWS_REGION）。
    pub fn new() -> anyhow::Result<Self> {
        let credentials = AwsCredentials::from_env()?;
        Ok(Self::with_credentials(credentials))
    }

    /// 从显式凭证创建。
    pub fn with_credentials(credentials: AwsCredentials) -> Self {
        Self {
            credentials,
            max_tokens: DEFAULT_MAX_TOKENS,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// 设置 max_tokens。
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    fn converse_url(&self, model_id: &str) -> String {
        format!(
            "https://{}.{}/model/{}/converse",
            ENDPOINT_PREFIX,
            "amazonaws.com",
            url_encode(model_id)
        )
    }

    fn convert_messages(messages: &[ChatMessage]) -> (Vec<ConverseMessage>, Vec<TextBlock>) {
        let mut system_blocks = Vec::new();
        let mut converse_messages = Vec::new();

        for m in messages {
            if m.role == "system" {
                system_blocks.push(TextBlock {
                    text: m.content.clone(),
                });
                continue;
            }
            converse_messages.push(ConverseMessage {
                role: m.role.clone(),
                content: vec![BedrockContentBlock::Text(TextBlock {
                    text: m.content.clone(),
                })],
            });
        }

        (converse_messages, system_blocks)
    }

    fn convert_tools(tools: Option<&[ToolSpec]>) -> Option<ToolConfig> {
        let items = tools?;
        if items.is_empty() {
            return None;
        }
        Some(ToolConfig {
            tools: items
                .iter()
                .map(|tool| BedrockTool {
                    tool_spec: BedrockToolSpec {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        input_schema: InputSchema {
                            json: tool.parameters.clone(),
                        },
                    },
                })
                .collect(),
        })
    }

    async fn converse(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolSpec]>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let (converse_messages, system_blocks) = Self::convert_messages(messages);
        let request = ConverseRequest {
            messages: converse_messages,
            system: if system_blocks.is_empty() {
                None
            } else {
                Some(system_blocks)
            },
            inference_config: Some(InferenceConfig {
                max_tokens: self.max_tokens,
                temperature,
            }),
            tool_config: Self::convert_tools(tools),
        };

        let payload = serde_json::to_vec(&request)?;
        let url = self.converse_url(model);
        let timestamp = Utc::now();

        let host = format!("{}.amazonaws.com", ENDPOINT_PREFIX);
        let amz_date = timestamp.format("%Y%m%dT%H%M%SZ").to_string();

        // 构建 headers（必须按小写名排序）
        let mut headers: Vec<(String, String)> = vec![
            ("host".to_string(), host.clone()),
            ("x-amz-date".to_string(), amz_date.clone()),
        ];
        if let Some(token) = &self.credentials.session_token {
            headers.push(("x-amz-security-token".to_string(), token.clone()));
        }
        headers.sort_by(|a, b| a.0.cmp(&b.0));

        let authorization = build_authorization_header(
            &self.credentials,
            "POST",
            &format!("/model/{}/converse", url_encode(model)),
            "", // 无 query string
            &headers,
            &payload,
            &timestamp,
        );

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", authorization);
        for (k, v) in &headers {
            req = req.header(k, v);
        }

        let response = req.body(payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Bedrock API error ({status}): {body}");
        }

        let converse_response: ConverseResponse = response.json().await?;
        let usage = converse_response.usage.map(|u| TokenUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
        });

        let content = converse_response
            .output
            .and_then(|o| o.message)
            .map(|m| m.content)
            .unwrap_or_default();

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        for block in content {
            match block {
                BedrockContentBlock::Text(t) => text_parts.push(t.text),
                BedrockContentBlock::ToolUse(tu) => {
                    let args = serde_json::to_string(&tu.tool_use.input).unwrap_or_default();
                    tool_calls.push(ProviderToolCall {
                        id: tu.tool_use.tool_use_id,
                        name: tu.tool_use.name,
                        arguments: args,
                    });
                }
                BedrockContentBlock::ToolResult(_) => {}
            }
        }

        Ok(ProviderChatResponse {
            text: if text_parts.is_empty() {
                None
            } else {
                Some(text_parts.join(""))
            },
            tool_calls,
            usage,
            reasoning_content: None,
            quota_metadata: None,
        })
    }
}

impl Default for BedrockProvider {
    fn default() -> Self {
        Self::new().expect("Bedrock requires AWS credentials")
    }
}

/// 简单的 URL 编码（仅处理 / 等特殊字符，用于 model_id）。
fn url_encode(s: &str) -> String {
    s.replace('/', "%2F")
}

#[async_trait]
impl Provider for BedrockProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(message));

        let response = self.converse(&messages, None, model, temperature).await?;
        Ok(response.text.unwrap_or_default())
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let response = self.converse(messages, None, model, temperature).await?;
        Ok(response.text.unwrap_or_default())
    }

    async fn chat(
        &self,
        request: ProviderChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        self.converse(request.messages, request.tools, model, temperature)
            .await
    }

    fn supports_native_tools(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_from_explicit() {
        let cred = AwsCredentials::new("AKIA", "secret", "us-west-2");
        assert_eq!(cred.access_key_id, "AKIA");
        assert_eq!(cred.region, "us-west-2");
        assert!(cred.session_token.is_none());
    }

    #[test]
    fn credentials_with_session_token() {
        let cred = AwsCredentials::new("AKIA", "secret", "us-east-1").with_session_token("session");
        assert_eq!(cred.session_token.as_deref(), Some("session"));
    }

    #[test]
    fn signing_key_derivation() {
        let key = derive_signing_key("secret", "20250101", "us-east-1", "bedrock");
        assert!(!key.is_empty());
    }

    #[test]
    fn authorization_header_format() {
        let cred = AwsCredentials::new("AKIATEST", "secrettest", "us-east-1");
        let ts = Utc::now();
        let headers = vec![
            (
                "host".to_string(),
                "bedrock-runtime.amazonaws.com".to_string(),
            ),
            (
                "x-amz-date".to_string(),
                ts.format("%Y%m%dT%H%M%SZ").to_string(),
            ),
        ];
        let auth = build_authorization_header(
            &cred,
            "POST",
            "/model/x/converse",
            "",
            &headers,
            b"{}",
            &ts,
        );
        assert!(auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIATEST/"));
        assert!(auth.contains("Signature="));
    }

    #[test]
    fn convert_messages_separates_system() {
        let msgs = vec![ChatMessage::system("be helpful"), ChatMessage::user("hi")];
        let (converse, system) = BedrockProvider::convert_messages(&msgs);
        assert_eq!(system.len(), 1);
        assert_eq!(system[0].text, "be helpful");
        assert_eq!(converse.len(), 1);
        assert_eq!(converse[0].role, "user");
    }

    #[test]
    fn convert_tools_handles_empty() {
        assert!(BedrockProvider::convert_tools(None).is_none());
        assert!(BedrockProvider::convert_tools(Some(&[])).is_none());
    }

    #[test]
    fn convert_tools_builds_config() {
        let tools = vec![ToolSpec {
            name: "shell".to_string(),
            description: "run".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let config = BedrockProvider::convert_tools(Some(&tools)).unwrap();
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.tools[0].tool_spec.name, "shell");
    }

    #[test]
    fn url_encode_handles_slash() {
        assert_eq!(
            url_encode("anthropic.claude-3-5-sonnet-20241022-v2:0"),
            "anthropic.claude-3-5-sonnet-20241022-v2:0"
        );
    }

    #[test]
    fn sha256_hex_works() {
        let hash = sha256_hex(b"hello");
        assert_eq!(hash.len(), 64); // 32 bytes hex
    }
}
