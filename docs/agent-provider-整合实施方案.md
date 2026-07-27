# LiteFlow-Rust Agent 多平台 Provider 整合实施方案

> **目标**：把 zeroclaw 经验证的 LLM Provider 实现整合进 `liteflow-rust/liteflow-agent`，
> 使其能对接 Java `liteflow-react-agent` 涵盖的全部平台，并补全 Rust 端缺失能力。
>
> **整合方式**：代码复制（非 crate 依赖），遵循 zeroclaw 的 `MIT OR Apache-2.0` 双许可，
> 本项目按 **Apache-2.0** 接入。
>
> **本文档基于实际代码调研**，所有文件路径、行号、依赖关系均经核实。

---

## 一、架构决策（已与维护者确认）

### 1.1 双层结构

| 平台分类 | 实现来源 | 现状 |
| --- | --- | --- |
| OpenAI / Anthropic / Gemini / DashScope / Ollama | **agentscope-rust 原生** `*ChatModel` | 现有 stub 薄包装，补全字段即可 |
| GLM / Copilot / Bedrock / OpenRouter / Telnyx / compatible / Codex | **zeroclaw Provider 代码复制** | 新建 crate |

### 1.2 目标 crate 结构

```
liteflow-agent/
├── liteflow-agent-core/            # 现有：ReAct 组件 + 配置 + 会话
├── liteflow-agent-openai/          # 现有 → 补全 GenerateOptions/reasoning
├── liteflow-agent-anthropic/       # 现有 → 补全 thinking
├── liteflow-agent-gemini/          # 现有（agentscope 原生，保持）
├── liteflow-agent-dashscope/       # 现有（agentscope 原生，保持）
├── liteflow-agent-ollama/          # 【新建】薄包装 agentscope ollama
│
├── liteflow-agent-provider-core/   # 【新建】轻量公共 crate
│   │   只放：zeroclaw Provider trait + ChatMessage/Response/StreamChunk +
│   │        ToolSpec + QuotaMetadata + ProviderToModelAdapter（Provider→Model 桥接）
│   └── 所有 zeroclaw 独有平台 crate 共同依赖它
│
├── liteflow-agent-glm/             # 【新建】智谱 GLM（JWT 原生认证）
├── liteflow-agent-copilot/         # 【新建】GitHub Copilot（OAuth 设备流）
├── liteflow-agent-bedrock/         # 【新建】AWS Bedrock（SigV4 签名）
├── liteflow-agent-openrouter/      # 【新建】OpenRouter 聚合网关
├── liteflow-agent-telnyx/          # 【新建】Telnyx 推理
├── liteflow-agent-compatible/      # 【新建】通用 OpenAI 兼容兜底
└── liteflow-agent-codex/           # 【新建】OpenAI Codex（Responses API，二期）
```

### 1.3 关键技术决策

| 决策点 | 选择 | 理由 |
| --- | --- | --- |
| 5 个 agentscope 已有平台 | 只用 agentscope 原生 `*ChatModel` | 已有完整流式/工具调用实现，不重复造轮子 |
| zeroclaw 独有平台 | 复制 zeroclaw Provider 代码 | 经生成验证，避免重写 HTTP/认证逻辑 |
| 公共 Provider trait 位置 | 轻量 `liteflow-agent-provider-core` crate | DRY，7 个平台共享一套 trait + adapter |
| trait 桥接方式 | `ProviderToModelAdapter` impl `agentscope_core::Model` | 在 adapter 内把 zeroclaw Provider 包装成 Model |
| GLM 认证 | zeroclaw JWT 原生 | 比 Java 走 OpenAI 兼容更准确 |
| Ollama | agentscope 原生（非 zeroclaw） | agentscope 已有，统一来源 |
| Codex | 二期（依赖 auth 子系统） | 深耦合 `crate::auth::AuthService`，单独攻坚 |

---

## 二、许可证合规（已完成）

### 已完成的工作

- ✅ `/NOTICE` — 项目 NOTICE，含 zeroclaw + agentscope 归属
- ✅ `/THIRD_PARTY_NOTICES.md` — 完整第三方清单（zeroclaw 文件映射表）
- ✅ `/README.md` 许可证章节 — 增加第三方代码归属说明

### 复制代码时必须遵守的规则

**每个从 zeroclaw 复制的 `.rs` 文件，头部必须加 prominent modification notice**：

```rust
// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
//
// 本文件衍生自 ZeroClaw 项目，遵循其 Apache-2.0 许可（ZeroClaw 为 MIT OR Apache-2.0 双许可）。
// "ZeroClaw" 是 ZeroClaw Labs 的商标；本项目与其无官方关联。
```

**商标限制**（TRADEMARK.md）：新 crate 不得命名为 `zeroclaw-*`；可用 `liteflow-agent-*` 前缀。
README/NOTICE 中可做描述性归属（"derived from ZeroClaw"）。

---

## 三、zeroclaw 代码依赖分析（调研结论）

### 3.1 各 provider 的内部依赖矩阵

| zeroclaw 文件 | 行数 | 依赖 crate::auth | 依赖 crate::config(proxy) | 依赖 crate::multimodal | 依赖 crate::tools::ToolSpec | 难度 |
| --- | --- | --- | --- | --- | --- | --- |
| `providers/traits.rs` | 964 | ❌ | ❌ | ❌ | ✅(ToolSpec) | 内核 |
| `tools/traits.rs` (ToolSpec) | 121 | ❌ | ❌ | ❌ | ❌ | 内核 |
| `providers/quota_types.rs` | 145 | ❌ | ❌ | ❌ | ❌ | 内核 |
| `providers/telnyx.rs` | 391 | ❌ | ✅ | ❌ | ❌ | 极易 |
| `providers/glm.rs` | 361 | ❌ | ✅ | ❌ | ❌ | 易 |
| `providers/openrouter.rs` | 1061 | ❌ | ✅ | ✅ | ✅ | 中 |
| `providers/bedrock.rs` | 2278 | ❌ | ✅ | ❌ | ✅ | 中 |
| `providers/copilot.rs` | 739 | ❌(自含OAuth) | ✅ | ❌ | ✅ | 中 |
| `providers/compatible.rs` | 3597 | ❌ | ✅ | ✅ | ✅ | 中(体积大) |
| `providers/openai_codex.rs` | 1604 | **✅(深耦合)** | ✅ | ✅ | ✅ | **难(二期)** |
| `providers/gemini.rs` | 2145 | ✅(深耦合) | ✅ | ❌ | ❌ | 不复制(用agentscope) |
| `providers/openai.rs` | 861 | ❌ | ✅ | ❌ | ✅ | 不复制(用agentscope) |
| `providers/anthropic.rs` | 1423 | ❌ | ✅ | ✅ | ✅ | 不复制(用agentscope) |
| `providers/ollama.rs` | 1075 | ❌ | ✅ | ✅ | ❌ | 不复制(用agentscope) |

### 3.2 import 路径转换表（全局替换规则）

复制 zeroclaw 文件后，需要做以下 import 路径替换：

| zeroclaw 原路径 | 新路径（在 liteflow-agent-provider-core 内） |
| --- | --- |
| `use crate::tools::ToolSpec;` | `use crate::tool_spec::ToolSpec;` |
| `use crate::providers::traits::{...};` | `use crate::traits::{...};`（同 crate 内）或 `use liteflow_agent_provider_core::traits::{...};`（跨 crate） |
| `use crate::providers::traits::build_tool_instructions_text;` | `use crate::traits::build_tool_instructions_text;` |
| `use crate::providers::ChatMessage;` | `use liteflow_agent_provider_core::ChatMessage;` |
| `crate::config::build_runtime_proxy_client_with_timeouts(...)` | `crate::proxy::build_proxy_client_with_timeouts(...)`（见 §5） |
| `crate::config::apply_runtime_proxy_to_builder(...)` | `crate::proxy::apply_proxy_to_builder(...)` |
| `use crate::multimodal;` / `crate::multimodal::parse_image_markers` | `use crate::multimodal;`（多模态代码复制到 provider-core 或各 crate 内） |

### 3.3 proxy 依赖的解耦策略

zeroclaw 几乎所有 provider 都调用 `crate::config::build_runtime_proxy_client_with_timeouts(service_key, timeout, connect_timeout)`。
该函数（`zeroclaw/src/config/schema.rs:2371`）功能：构建带 timeout + 全局代理配置的 `reqwest::Client`，并缓存。

**解耦方案**：在 `liteflow-agent-provider-core/src/proxy.rs` 新写一个**简化版**（~60 行），
去掉全局 `OnceLock<RwLock<...>>` 状态（liteflow 不需要 zeroclaw 的运行时代理热更新）：

```rust
//! 简化版代理客户端构建（衍生自 ZeroClaw config/schema.rs，Apache-2.0）。
//! 去掉了 ZeroClaw 的全局运行时代理状态热更新，改为构建时一次性配置。
use std::time::Duration;
use reqwest::Client;

/// 构建带超时的 reqwest 客户端（可选代理）。
pub fn build_client_with_timeouts(
    timeout_secs: u64,
    connect_timeout_secs: u64,
    proxy_url: Option<&str>,
) -> Client {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(connect_timeout_secs));
    if let Some(url) = proxy_url {
        if let Ok(proxy) = reqwest::Proxy::all(url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_else(|_| Client::new())
}
```

zeroclaw provider 中的 `crate::config::build_runtime_proxy_client_with_timeouts("provider.X", 120, 10)`
统一替换为 `crate::proxy::build_client_with_timeouts(120, 10, None)`（代理 URL 通过配置传入）。

---

## 四、liteflow-agent-provider-core（公共 crate）详细设计

### 4.1 文件清单

```
liteflow-agent-provider-core/
├── Cargo.toml
└── src/
    ├── lib.rs          # 模块声明 + re-export
    ├── tool_spec.rs    # 复制自 zeroclaw tools/traits.rs（仅 ToolSpec，121行）
    ├── traits.rs       # 复制自 zeroclaw providers/traits.rs（Provider trait 等，964行）
    ├── quota_types.rs  # 复制自 zeroclaw providers/quota_types.rs（145行）
    ├── proxy.rs        # 新写：简化版代理客户端构建（~60行）
    ├── multimodal.rs   # 复制自 zeroclaw multimodal.rs（690行，按需）
    └── adapter.rs      # 新写：ProviderToModelAdapter（Provider→Model 桥接，~200行）
```

### 4.2 Cargo.toml

```toml
[package]
name = "liteflow-agent-provider-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Core Provider trait and Model adapter for liteflow-agent (derived from ZeroClaw, Apache-2.0)"

[dependencies]
agentscope-core = { path = "../../../../workspace-octoclaw-labs/agentscope-rust/crates/agentscope-core" }
async-trait.workspace = true
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true
serde_json.workspace = true
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream", "socks"] }
futures-util = { version = "0.3", default-features = false, features = ["sink"] }
# multimodal.rs 按需启用：
base64 = { version = "0.22", optional = true }
image = { version = "0.25", default-features = false, features = ["jpeg", "png"], optional = true }

[features]
default = []
multimodal = ["dep:base64", "dep:image"]
```

### 4.3 lib.rs

```rust
//! LiteFlow Agent Provider Core — 衍生自 ZeroClaw 的 Provider trait 体系。
//!
//! 本 crate 提供：
//! - zeroclaw `Provider` trait 及其消息/响应类型（ChatMessage/ChatResponse/StreamChunk 等）
//! - `ProviderToModelAdapter`：把任意 `Provider` 包装成 `agentscope_core::Model`
//!
//! 各平台子 crate（liteflow-agent-glm/copilot/bedrock 等）依赖本 crate，
//! 复用 Provider trait 与 adapter，避免重复。

pub mod tool_spec;
pub mod traits;
pub mod quota_types;
pub mod proxy;
pub mod adapter;
#[cfg(feature = "multimodal")]
pub mod multimodal;

pub use tool_spec::ToolSpec;
pub use traits::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider,
    ProviderCapabilityError, ProviderCapabilities, StreamChunk, StreamError,
    StreamOptions, StreamResult, TokenUsage, ToolCall, ToolResultMessage, ToolsPayload,
};
pub use adapter::ProviderToModelAdapter;
```

### 4.4 adapter.rs 核心实现（Provider → Model 桥接）

这是整个整合的技术核心。zeroclaw `Provider` 是同步/单次请求抽象，
agentscope `Model` 是流式抽象。adapter 负责转换。

**agentscope 实际 API（已核实）**：
- `ContentBlock` enum 变体：`Text(TextBlock)` / `ToolUse(ToolUseBlock)` / `Image` / `Thinking` 等
  （位于 `agentscope-core/src/message/content_block.rs`，**没有 `ToolCall` 变体，是 `ToolUse`**）
- `ToolUseBlock` 字段：`id: String` / `name: String` / `input: HashMap<String, Value>`
  （位于 `agentscope-core/src/message/tool_use.rs:68`，有 `ToolUseBlock::new(id, name, input)` 构造器）
- `TextBlock` 字段：`text: String`（可直接 `TextBlock { text }` 构造）
- `Msg` 方法：`role() -> MsgRole`、`content() -> &[ContentBlock]`（**注意是 `content()` 不是 `content_blocks()`**）
- `ModelError` 变体：有 `Other(String)`、`Network(reqwest::Error)`、`Provider { ... }` 等
- `ChatResponse::builder().content(Vec<ContentBlock>).model_name(String).build()`

```rust
//! ProviderToModelAdapter — 把 zeroclaw Provider 包装成 agentscope Model。
//!
//! 转换逻辑：
//! - agentscope `Msg` → zeroclaw `ChatMessage`（提取 role + 文本 content）
//! - agentscope `ToolSchema` → zeroclaw `ToolSpec`（字段几乎一一对应）
//! - zeroclaw `ChatResponse` → agentscope `ChatResponse`
//!   （text→ContentBlock::Text(TextBlock)，tool_calls→ContentBlock::ToolUse(ToolUseBlock)）
//! - 单次 Provider.chat() 结果 → 单元素 Stream（满足 Model.stream() 签名）

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use agentscope_core::{
    message::{ContentBlock, TextBlock, ToolUseBlock},
    model::{ChatResponse as AsChatResponse, GenerateOptions, ModelError, ToolSchema},
    Model, Msg, MsgRole,
};
use futures_util::stream::{self, Stream};
use serde_json::Value;

use crate::traits::{ChatMessage, Provider};

/// 任意 `Provider` 的 `Model` 适配器。
pub struct ProviderToModelAdapter {
    name: String,
    model_name: String,
    provider: Arc<dyn Provider>,
    temperature: f64,
}

impl ProviderToModelAdapter {
    /// 构造适配器。
    ///
    /// - `name`：显示名（如 "glm-4.6"）
    /// - `model_name`：传给 Provider 的模型 ID
    /// - `provider`：zeroclaw Provider 实例（如 GlmProvider::new(...)）
    /// - `temperature`：默认采样温度
    pub fn new(
        name: impl Into<String>,
        model_name: impl Into<String>,
        provider: Arc<dyn Provider>,
        temperature: f64,
    ) -> Self {
        Self {
            name: name.into(),
            model_name: model_name.into(),
            provider,
            temperature,
        }
    }
}

impl Model for ProviderToModelAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn stream(
        &self,
        messages: &[Msg],
        tools: &[ToolSchema],
        _options: Option<&GenerateOptions>,
    ) -> Pin<Box<dyn Stream<Item = Result<AsChatResponse, ModelError>> + Send>> {
        // 1. Msg → ChatMessage（Clone 成 owned，因为 async block 需 'static）
        let zc_messages: Vec<ChatMessage> = messages.iter().map(msg_to_chat_message).collect();

        // 2. ToolSchema → serde_json::Value（OpenAI tools 格式，zeroclaw chat() 接受）
        let zc_tools: Vec<Value> = tools.iter().map(tool_schema_to_json).collect();

        // 3. Clone 参数进 async block
        let provider = Arc::clone(&self.provider);
        let model_name = self.model_name.clone();
        let temperature = self.temperature;

        // 4. 调用 Provider.chat()，把结果包成单元素 Stream
        //    注意：Provider::chat(ChatRequest<'a>) 借用 messages，但 Stream 需 'static。
        //    解决方案见 §4.5：需在 traits.rs 加 owned 版本 chat 方法。
        Box::pin(stream::once(async move {
            let request = crate::traits::ChatRequest {
                messages: &zc_messages,  // zc_messages 已 move 进 block，借用 OK
                tools: if zc_tools.is_empty() { None } else { Some(&zc_tools) },
            };
            let result = provider
                .chat(request, &model_name, temperature)
                .await
                .map_err(|e| ModelError::Other(format!("{e:?}")))?;
            Ok(zc_response_to_as_response(result, &model_name))
        }))
    }
}

/// Msg → ChatMessage：提取 role + 纯文本 content。
/// 注意：multimodal content（图片/音频）暂不支持，标注为已知限制。
fn msg_to_chat_message(msg: &Msg) -> ChatMessage {
    let role = match msg.role() {
        MsgRole::System => "system",
        MsgRole::User => "user",
        MsgRole::Assistant => "assistant",
        MsgRole::Tool => "tool",
    };
    // Msg::content() 返回 &[ContentBlock]，提取所有 Text 块拼接
    let content: String = msg
        .content()
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None, // 图片/音频等暂跳过（已知限制）
        })
        .collect::<Vec<_>>()
        .join("");
    ChatMessage {
        role: role.to_string(),
        content,
    }
}

/// ToolSchema → serde_json::Value（OpenAI tools 格式）。
fn tool_schema_to_json(schema: &ToolSchema) -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": schema.name,
            "description": schema.description,
            "parameters": schema.parameters,
        }
    })
}

/// zeroclaw ChatResponse → agentscope ChatResponse。
fn zc_response_to_as_response(zc: crate::traits::ChatResponse, model_name: &str) -> AsChatResponse {
    let mut content = Vec::new();
    if let Some(text) = zc.text {
        if !text.is_empty() {
            content.push(ContentBlock::Text(TextBlock { text }));
        }
    }
    for tc in zc.tool_calls {
        // zeroclaw ToolCall.arguments 是 String(JSON)，agentscope ToolUseBlock.input 是 HashMap
        let input: HashMap<String, Value> = serde_json::from_str(&tc.arguments)
            .unwrap_or_default();
        content.push(ContentBlock::ToolUse(ToolUseBlock::new(tc.id, tc.name, input)));
    }
    AsChatResponse::builder()
        .content(content)
        .model_name(model_name.to_string())
        .build()
}
```

### 4.5 adapter 实现的关键注意点（已核实 agentscope 实际 API）

1. **生命周期问题**：zeroclaw `Provider::chat(ChatRequest<'a>, ...)` 的 `ChatRequest` 借用 messages。
   但 agentscope `Model::stream()` 返回 `'static` Stream。**解决方案**（推荐）：
   在 provider-core 的 `traits.rs` 里给 Provider trait 加一个 owned 版本默认方法：
   ```rust
   async fn chat_owned(
       &self,
       messages: Vec<ChatMessage>,   // owned，不借用
       tools: Option<Vec<serde_json::Value>>,
       model: &str,
       temperature: f64,
   ) -> anyhow::Result<ChatResponse> {
       let req = ChatRequest { messages: &messages, tools: tools.as_deref() };
       self.chat(req, model, temperature).await
   }
   ```
   adapter 调用 `chat_owned(zc_messages, ...)` 即可满足 'static 约束。

2. **`ContentBlock` 变体（已确认）**：agentscope 是 `ToolUse(ToolUseBlock)`，
   **不是** `ToolCall`。`ToolUseBlock::new(id: String, name: String, input: HashMap<String, Value>)`。
   zeroclaw `ToolCall.arguments` 是 `String`(JSON)，需 `serde_json::from_str` 解析成 HashMap。

3. **`Msg` 方法（已确认）**：是 `msg.content() -> &[ContentBlock]`，**不是** `content_blocks()`。
   `TextBlock` 字段是 `text: String`（公开字段，可直接构造）。

4. **流式 vs 单次**：adapter 当前把单次 `chat()` 结果包成单元素 Stream。
   若需真流式，可调用 `provider.stream_chat_with_history()` 并把 `StreamChunk` 逐个转 `ChatResponse`
   （`StreamChunk.delta` → `TextBlock`，`is_final` 时结束流）。

5. **错误类型映射（已确认）**：agentscope `ModelError` 有 `Other(String)` 变体。
   zeroclaw `anyhow::Error` → `ModelError::Other(e.to_string())`。

---

## 五、各平台 crate 详细设计

### 5.1 通用 crate 模板（适用于 GLM/Copilot/Bedrock/OpenRouter/Telnyx/compatible）

每个平台 crate 结构与现有 `liteflow-agent-gemini` 同构：

```
liteflow-agent-<platform>/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── model/
    │   ├── mod.rs
    │   └── <platform>_agent_model_config.rs   # 配置 struct + build()
    └── provider.rs                             # 从 zeroclaw 复制的 provider 实现
```

**契约**（与现有 stub 一致，见 `liteflow-agent/tests/provider_contract.rs`）：
- `*AgentModelConfig` 必须 `derive(Serialize, Deserialize)` + `#[serde(rename_all = "camelCase")]`
- `new(api_key, model_name)` 两参构造器
- `build(&self) -> Arc<dyn Model>` 不触网、返回 `.name()` 非空的模型

### 5.2 liteflow-agent-glm（智谱 GLM JWT 原生）

**文件**：
| 新文件 | 来源 | 行数 | 修改 |
| --- | --- | --- | --- |
| `src/provider.rs` | zeroclaw `src/providers/glm.rs` | 361 | import 路径 + 版权头 |
| `src/model/glm_agent_model_config.rs` | 新写 | ~60 | 见下方代码 |

**Cargo.toml**：
```toml
[package]
name = "liteflow-agent-glm"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Zhipu GLM (JWT auth) model adapter for liteflow-agent"

[dependencies]
agentscope-core = { path = "../../../../workspace-octoclaw-labs/agentscope-rust/crates/agentscope-core" }
liteflow-agent-provider-core = { path = "../liteflow-agent-provider-core" }
async-trait.workspace = true
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde.workspace = true
serde_json.workspace = true
ring = "0.17"          # HMAC-SHA256 JWT 签名（zeroclaw glm.rs:8 依赖）
```

**glm_agent_model_config.rs**：
```rust
//! 智谱 GLM 模型配置（JWT 原生认证，衍生自 ZeroClaw）。
use std::sync::Arc;
use agentscope_core::Model;
use liteflow_agent_provider_core::ProviderToModelAdapter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlmAgentModelConfig {
    /// GLM API Key，格式 `id.secret`。
    pub api_key: String,
    /// 模型名称，如 `glm-4.6`。
    pub model_name: String,
    /// 可选网关地址（默认 `https://open.bigmodel.cn/api/paas/v4`）。
    pub base_url: Option<String>,
    /// 采样温度。
    pub temperature: f64,
}

impl GlmAgentModelConfig {
    #[must_use]
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            base_url: None,
            temperature: 0.7,
        }
    }

    /// 构建 `Arc<dyn Model>`（不触网：JWT 在首次请求时才生成）。
    #[must_use]
    pub fn build(&self) -> Arc<dyn Model> {
        // 拆分 api_key 为 id.secret
        let (id, secret) = self.api_key.split_once('.').unwrap_or((&self.api_key, ""));
        let base_url = self.base_url.clone().unwrap_or_else(|| 
            "https://open.bigmodel.cn/api/paas/v4".to_string());
        let provider = crate::provider::GlmProvider::new(id, secret, &base_url);
        Arc::new(ProviderToModelAdapter::new(
            self.model_name.clone(),
            self.model_name.clone(),
            Arc::new(provider),
            self.temperature,
        ))
    }
}
```

### 5.3 liteflow-agent-copilot（GitHub OAuth 设备流）

| 新文件 | 来源 | 行数 | 特殊依赖 |
| --- | --- | --- | --- |
| `src/provider.rs` | zeroclaw `src/providers/copilot.rs` | 739 | `directories`(token缓存)、`tokio`(OAuth loopback)、`tracing` |
| `src/model/copilot_agent_model_config.rs` | 新写 | ~50 | — |

**注意**：copilot 的 OAuth 是**自包含的**（不走 crate::auth），VS Code client_id 硬编码在 zeroclaw `copilot.rs:30`。
token 缓存到 `directories::ProjectDirs`。首次使用需设备码流交互，`build()` 本身不触网（只构造 provider）。

**Cargo.toml 额外依赖**：`directories = "6.0"`, `tokio = { workspace = true }`, `tracing = "0.1"`

### 5.4 liteflow-agent-bedrock（AWS SigV4）

| 新文件 | 来源 | 行数 | 特殊依赖 |
| --- | --- | --- | --- |
| `src/provider.rs` | zeroclaw `src/providers/bedrock.rs` | 2278 | `hmac`、`sha2`（SigV4 签名）、`chrono` |
| `src/model/bedrock_agent_model_config.rs` | 新写 | ~70 | — |

**配置字段**：`region`、`access_key_id`、`secret_access_key`、`model_name`（如 `anthropic.claude-3-5-sonnet-20241022-v2:0`）。
**Cargo.toml 额外依赖**：`hmac = "0.12"`, `sha2 = "0.10"`, `chrono = { version = "0.4", features = ["clock","std","serde"] }`

### 5.5 liteflow-agent-openrouter

| 新文件 | 来源 | 行数 |
| --- | --- | --- |
| `src/provider.rs` | zeroclaw `src/providers/openrouter.rs` | 1061 |
| `src/model/openrouter_agent_model_config.rs` | 新写 | ~50 |

**配置字段**：`api_key`、`model_name`（如 `anthropic/claude-3.5-sonnet`）、`base_url`(可选，默认 `https://openrouter.ai/api/v1`)。
需启用 provider-core 的 `multimodal` feature（openrouter 用了 `crate::multimodal`）。

### 5.6 liteflow-agent-telnyx

| 新文件 | 来源 | 行数 |
| --- | --- | --- |
| `src/provider.rs` | zeroclaw `src/providers/telnyx.rs` | 391 |
| `src/model/telnyx_agent_model_config.rs` | 新写 | ~45 |

**最干净的 provider**（零内部依赖）。配置字段：`api_key`、`model_name`、`base_url`(可选)。

### 5.7 liteflow-agent-compatible（通用 OpenAI 兼容兜底）

| 新文件 | 来源 | 行数 |
| --- | --- | --- |
| `src/provider.rs` | zeroclaw `src/providers/compatible.rs` | 3597 |
| `src/model/compatible_agent_model_config.rs` | 新写 | ~60 |

**用途**：覆盖 DeepSeek/Kimi/Minimax/Qwen/Groq/Mistral/xAI 等所有 OpenAI 兼容服务。
**配置字段**：`api_key`、`base_url`（必填）、`model_name`、`auth_style`(Bearer/Custom)、`native_tool_calling`(bool)。
**注意**：compatible.rs 用了 `tokio-tungstenite`（websocket 流式），需加依赖 `tokio-tungstenite = { version = "0.28", features = ["rustls-tls-webpki-roots"] }`。
体积大（3597行），建议精简：若不需要 websocket，可删除相关代码段。

### 5.8 liteflow-agent-codex（二期，依赖 auth 子系统）

**暂缓实现**。zeroclaw `openai_codex.rs` 深度依赖 `crate::auth::AuthService`（OAuth token 管理）。
若要实现，需额外复制 zeroclaw `src/auth/`（6 文件 2585 行）+ `src/security/secrets.rs`（878 行），
并重构 `AuthService::from_config` 解耦对 zeroclaw `Config`（12587 行）的依赖。
建议：先用 compatible crate 覆盖 Codex 的 OpenAI 兼容接口，原生 Responses API 留作二期。

---

## 六、现有 5 个 stub 的增强（用 agentscope 原生能力补全）

### 6.1 liteflow-agent-openai（补全 GenerateOptions）

**现状缺口**（`open_ai_agent_model_config.rs:14-25`）：只有 `api_key/model_name/base_url/endpoint_path/stream`，
**缺少** temperature/topP/maxTokens/seed/reasoning_effort 等。

**Java 对照**（`OpenAISpec.java`）：基类 `ModelSpec` 持有 temperature/topP/topK/maxTokens/seed/stream/cacheControl，
子类额外有 reasoningEffort/frequencyPenalty/presencePenalty，通过 `buildGenerateOptions()` 装配。

**修改**：增加 `GenerateOptions` 字段并传给 builder：
```rust
// open_ai_agent_model_config.rs 增加：
pub temperature: Option<f64>,
pub top_p: Option<f64>,
pub max_tokens: Option<u64>,
pub seed: Option<u64>,
pub reasoning_effort: Option<String>,  // "low"/"medium"/"high"（o1/o3 系列）

// build() 内：
if let Some(temp) = self.temperature {
    builder = builder.generate_options(
        GenerateOptions::builder().temperature(temp).build()
    );
}
// reasoning_effort 通过 metadata 传递（需确认 OpenAIChatModel builder 支持）
```

### 6.2 liteflow-agent-anthropic（补全 thinking）

**现状缺口**：缺少 `thinking_budget`/`thinking_enabled`（Claude 思考模式）。

**Java 对照**（`AnthropicSpec.java:44-50`）：`thinking(Consumer<AnthropicThinking>)` 设置 budget/enabled。

**修改**：增加 `thinking_budget: Option<u64>` / `thinking_enabled: Option<bool>` 字段，
通过 `AnthropicChatModel::builder().generate_options(GenerateOptions::builder().thinking_budget(...).build())` 传递。

### 6.3 liteflow-agent-gemini / dashscope

agentscope 原生实现较完整，**保持现状**。仅检查是否需补全 `GenerateOptions` 传递路径（与 openai 同理）。

### 6.4 liteflow-agent-ollama（新建，薄包装 agentscope）

agentscope 已有 `OllamaChatModel`（`agentscope-core/src/model/ollama.rs`），与现有 4 个 stub 完全同构：
```toml
# Cargo.toml
[dependencies]
agentscope-core = { path = "..." }
serde.workspace = true
```
```rust
// ollama_agent_model_config.rs
pub struct OllamaAgentModelConfig {
    pub model_name: String,
    pub base_url: Option<String>,  // 默认 http://localhost:11434
    pub stream: bool,
}
impl OllamaAgentModelConfig {
    pub fn build(&self) -> Arc<dyn Model> {
        let mut builder = OllamaChatModel::builder()
            .model_name(&self.model_name)
            .stream(self.stream);
        if let Some(url) = &self.base_url { builder = builder.base_url(url); }
        Arc::new(builder.build())
    }
}
```

---

## 七、顶层集成

### 7.1 根 workspace Cargo.toml（`liteflow-rust/Cargo.toml`）

在 `members` 数组增加全部新 crate：
```toml
members = [
    # ... 现有 ...
    "liteflow-agent/liteflow-agent-provider-core",
    "liteflow-agent/liteflow-agent-ollama",
    "liteflow-agent/liteflow-agent-glm",
    "liteflow-agent/liteflow-agent-copilot",
    "liteflow-agent/liteflow-agent-bedrock",
    "liteflow-agent/liteflow-agent-openrouter",
    "liteflow-agent/liteflow-agent-telnyx",
    "liteflow-agent/liteflow-agent-compatible",
]
```

在 `[workspace.dependencies]` 增加新依赖：
```toml
async-trait = "0.1"
anyhow = "1.0"
thiserror = "2.0"
futures-util = { version = "0.3", default-features = false, features = ["sink"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream", "socks"] }
ring = "0.17"
hmac = "0.12"
sha2 = "0.10"
chrono = { version = "0.4", features = ["clock", "std", "serde"] }
base64 = "0.22"
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
directories = "6.0"
tokio-tungstenite = { version = "0.28", features = ["rustls-tls-webpki-roots"] }
tracing = "0.1"
```

### 7.2 liteflow-agent/Cargo.toml（feature 编排）

```toml
[features]
default = ["core"]
core = ["dep:liteflow-agent-core"]
# agentscope 原生 5 个
openai = ["core", "dep:liteflow-agent-openai"]
anthropic = ["core", "dep:liteflow-agent-anthropic"]
gemini = ["core", "dep:liteflow-agent-gemini"]
dashscope = ["core", "dep:liteflow-agent-dashscope"]
ollama = ["core", "dep:liteflow-agent-ollama"]
# zeroclaw 独有 6 个（依赖 provider-core）
provider-core = ["dep:liteflow-agent-provider-core"]
glm = ["provider-core", "dep:liteflow-agent-glm"]
copilot = ["provider-core", "dep:liteflow-agent-copilot"]
bedrock = ["provider-core", "dep:liteflow-agent-bedrock"]
openrouter = ["provider-core", "dep:liteflow-agent-openrouter"]
telnyx = ["provider-core", "dep:liteflow-agent-telnyx"]
compatible = ["provider-core", "dep:liteflow-agent-compatible"]

[dependencies]
# ... 现有 5 个 optional 依赖 ...
liteflow-agent-ollama = { path = "liteflow-agent-ollama", optional = true }
liteflow-agent-provider-core = { path = "liteflow-agent-provider-core", optional = true }
liteflow-agent-glm = { path = "liteflow-agent-glm", optional = true }
liteflow-agent-copilot = { path = "liteflow-agent-copilot", optional = true }
liteflow-agent-bedrock = { path = "liteflow-agent-bedrock", optional = true }
liteflow-agent-openrouter = { path = "liteflow-agent-openrouter", optional = true }
liteflow-agent-telnyx = { path = "liteflow-agent-telnyx", optional = true }
liteflow-agent-compatible = { path = "liteflow-agent-compatible", optional = true }
```

### 7.3 liteflow-agent/src/lib.rs（re-export）

```rust
// 现有 5 个 agentscope 原生 ...
#[cfg(feature = "ollama")]
pub use liteflow_agent_ollama as ollama;
#[cfg(feature = "ollama")]
pub use liteflow_agent_ollama::OllamaAgentModelConfig;

// zeroclaw 独有 6 个
#[cfg(feature = "provider-core")]
pub use liteflow_agent_provider_core as provider_core;
#[cfg(feature = "glm")]
pub use liteflow_agent_glm as glm;
#[cfg(feature = "glm")]
pub use liteflow_agent_glm::GlmAgentModelConfig;
// ... copilot/bedrock/openrouter/telnyx/compatible 同理 ...
```

### 7.4 契约测试（liteflow-agent/tests/provider_contract.rs）

为每个新平台加测试块（遵循现有模式）：
```rust
#[cfg(feature = "glm")]
mod glm {
    use liteflow_agent::glm::GlmAgentModelConfig;
    #[test]
    fn config_serializes_and_builds() {
        let config = GlmAgentModelConfig::new("test-id.test-secret", "glm-4.6");
        let json = serde_json::to_value(&config).expect("GLM config should serialize");
        assert_eq!(json["modelName"], "glm-4.6");
        assert!(!config.build().name().is_empty());  // 不触网，name 非空
    }
}
```

---

## 八、分阶段执行计划

### 阶段 A：MVP 验证（先跑通一条链路）
1. 建 `liteflow-agent-provider-core`：复制 traits.rs/tool_spec.rs/quota_types.rs + 新写 proxy.rs + adapter.rs
2. 建 `liteflow-agent-glm`：复制 glm.rs + 写 config
3. `cargo check -p liteflow-agent-provider-core -p liteflow-agent-glm`
4. **验证点**：adapter 能把 GlmProvider 包装成 Model，契约测试通过

### 阶段 B：批量复制其余 5 个平台
按难度顺序：telnyx(极易) → openrouter → copilot → bedrock → compatible
每复制一个 `cargo check` 验证。

### 阶段 C：现有 stub 增强
补全 openai/anthropic 的 GenerateOptions/thinking 字段。

### 阶段 D：顶层集成 + 全量验证
- 根 Cargo.toml 注册 + features
- lib.rs re-export
- 契约测试全覆盖
- `cargo check --workspace --all-features`
- `cargo test -p liteflow-agent --all-features`
- `cargo clippy --workspace`

### 阶段 E（二期）：codex
单独攻坚 auth 子系统。

---

## 九、风险与缓解

| 风险 | 缓解 |
| --- | --- |
| adapter 生命周期问题（Provider.chat 借用 vs Model.stream 返 'static） | 在 provider-core traits.rs 加 owned 版本 chat 方法，或 clone messages |
| `ContentBlock::ToolCall` 可能不存在于 agentscope | 实现前 grep 核实；若无则降级为 Text(JSON) |
| zeroclaw 代码体积大（compatible 3597 行） | 按需精简，删 websocket 段可减 ~500 行 |
| 外部 crate 版本冲突（ring/hmac 等） | workspace 统一版本，cargo check 排查 |
| codex 深耦合 auth | 二期单独处理，先用 compatible 覆盖 |

---

## 十、已完成项

- ✅ 三库深度调研（Java/Rust/zeroclaw）
- ✅ 许可证合规分析（zeroclaw 双许可 → Apache-2.0 接入）
- ✅ NOTICE + THIRD_PARTY_NOTICES.md + README 归属
- ✅ 本实施方案文档

## 待执行项清单（按阶段 A→D）

- [ ] 阶段 A：provider-core + glm（MVP）
- [ ] 阶段 B：telnyx/openrouter/copilot/bedrock/compatible
- [ ] 阶段 C：openai/anthropic stub 增强
- [ ] 阶段 D：顶层集成 + 全量验证
- [ ] 阶段 E（二期）：codex + auth 子系统
