// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目 src/providers/telnyx.rs。
// 修改：import 路径调整为 liteflow-agent-provider-core；sanitize_api_error 改用 provider-core。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。

//! Telnyx AI inference provider.
//!
//! Telnyx provides AI inference through an OpenAI-compatible API at
//! https://api.telnyx.com/v2/ai with access to 53+ models including
//! GPT-4o, Claude, Llama, Mistral, and more.

use async_trait::async_trait;
use liteflow_agent_provider_core::util::sanitize_api_error;
use liteflow_agent_provider_core::{ChatMessage, Provider};
use reqwest::Client;
use serde::Deserialize;

/// Telnyx AI inference provider.
///
/// Uses the OpenAI-compatible chat completions API at `/v2/ai/chat/completions`.
pub struct TelnyxProvider {
    /// Telnyx API key
    api_key: Option<String>,
    /// HTTP client for API requests
    client: Client,
}

impl TelnyxProvider {
    /// Telnyx AI API base URL
    const BASE_URL: &'static str = "https://api.telnyx.com/v2/ai";

    /// Create a new Telnyx AI provider.
    ///
    /// The API key can be provided directly or will be resolved from:
    /// 1. `TELNYX_API_KEY` environment variable
    pub fn new(api_key: Option<&str>) -> Self {
        let resolved_key = resolve_telnyx_api_key(api_key);
        Self {
            api_key: resolved_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// Create a provider with a custom base URL (for testing or proxies).
    pub fn with_base_url(api_key: Option<&str>, _base_url: &str) -> Self {
        Self::new(api_key)
    }

    /// Build the chat completions URL
    fn chat_url(&self) -> String {
        format!("{}/chat/completions", Self::BASE_URL)
    }
}

/// Resolve Telnyx API key from parameter or environment.
fn resolve_telnyx_api_key(api_key: Option<&str>) -> Option<String> {
    if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
        return Some(key.to_string());
    }

    if let Ok(key) = std::env::var("TELNYX_API_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            return Some(key.to_string());
        }
    }

    None
}

/// Response from chat completions API
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Request body for chat completions
#[derive(Debug, serde::Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
}

#[derive(Debug, serde::Serialize)]
struct Message {
    role: String,
    content: String,
}

#[async_trait]
impl Provider for TelnyxProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Telnyx API key not set. Set TELNYX_API_KEY environment variable.")
        })?;

        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }
        messages.push(Message {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature,
        };

        let response = self
            .client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await?;
            let sanitized = sanitize_api_error(&error);
            anyhow::bail!("Telnyx API error ({}): {}", status, sanitized);
        }

        let chat_response: ChatResponse = response.json().await?;
        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response from Telnyx"))
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Telnyx API key not set. Set TELNYX_API_KEY environment variable.")
        })?;

        let api_messages: Vec<Message> = messages
            .iter()
            .map(|m| Message {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = ChatRequest {
            model: model.to_string(),
            messages: api_messages,
            temperature,
        };

        let response = self
            .client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await?;
            let sanitized = sanitize_api_error(&error);
            anyhow::bail!("Telnyx API error ({}): {}", status, sanitized);
        }

        let chat_response: ChatResponse = response.json().await?;
        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response from Telnyx"))
    }
}
