# Third-Party Notices

This document lists third-party code incorporated into LiteFlow-Rust and
the licenses under which it is used.

## ZeroClaw

- **Project:** ZeroClaw (https://github.com/zeroclaw-labs/zeroclaw)
- **Copyright:** © 2025 ZeroClaw Labs
- **License:** MIT OR Apache-2.0 (dual-licensed). LiteFlow-Rust incorporates
  ZeroClaw code under the **Apache License 2.0**.
- **Used in:** `liteflow-agent/liteflow-agent-providers/`

### What was incorporated

ZeroClaw's `src/providers/` subsystem provides battle-tested LLM provider
implementations (HTTP clients, authentication flows, streaming, tool-calling)
for many model backends. LiteFlow-Rust adapts these into its agent
subsystem. The incorporated files and their integration status:

| ZeroClaw source file | Purpose | 整合状态 | 落地位置 |
| --- | --- | --- | --- |
| `src/providers/traits.rs` | `Provider` trait, `ChatMessage`/`ChatResponse`/`StreamChunk`/`ToolsPayload` | ✅ 完整复制（新增 `chat_owned`） | `liteflow-agent-provider-core/src/traits.rs` |
| `src/tools/traits.rs` (partial) | `ToolSpec` / `ToolResult` 类型 | ✅ 完整复制 | `liteflow-agent-provider-core/src/tool_spec.rs` |
| `src/providers/quota_types.rs` | Quota metadata 类型 | ✅ 完整复制 | `liteflow-agent-provider-core/src/quota_types.rs` |
| `src/providers/backoff.rs` | 指数退避存储 | ✅ 完整复制 | `liteflow-agent-provider-core/src/backoff.rs` |
| `src/providers/mod.rs` (partial) | `sanitize_api_error` / `scrub_secret_patterns` / `api_error` | ✅ 复制（脱敏工具） | `liteflow-agent-provider-core/src/util.rs` |
| `src/config/schema.rs` (partial) | `ProxyConfig` + 代理客户端构建 | ✅ 简化重写（去全局状态） | `liteflow-agent-provider-core/src/proxy.rs` |
| `src/providers/telnyx.rs` | Telnyx 推理 | ✅ 完整复制 | `liteflow-agent-telnyx/src/provider.rs` |
| `src/providers/glm.rs` | 智谱 GLM（JWT 认证） | ✅ 完整复制（修正 client bug） | `liteflow-agent-glm/src/provider.rs` |
| `src/providers/openrouter.rs` | OpenRouter 聚合网关 | ✅ 精简（去 multimodal 图片） | `liteflow-agent-openrouter/src/provider.rs` |
| `src/providers/copilot.rs` | GitHub Copilot（OAuth 设备流） | ✅ 完整复制 | `liteflow-agent-copilot/src/provider.rs` |
| `src/providers/bedrock.rs` | AWS Bedrock（SigV4 签名） | ✅ 精简（去 IMDS/图片，保留 SigV4+Converse） | `liteflow-agent-bedrock/src/provider.rs` |
| `src/providers/compatible.rs` | 通用 OpenAI 兼容兜底 | ✅ 精简（去 websocket/responses API） | `liteflow-agent-compatible/src/provider.rs` |
| `src/providers/openai.rs` | OpenAI Chat Completions | ⏭️ 未复制（用 agentscope 原生 `OpenAIChatModel`） | `liteflow-agent-openai/` |
| `src/providers/anthropic.rs` | Anthropic Messages API | ⏭️ 未复制（用 agentscope 原生 `AnthropicChatModel`） | `liteflow-agent-anthropic/` |
| `src/providers/gemini.rs` | Google Gemini (+ OAuth/ADC) | ⏭️ 未复制（用 agentscope 原生 `GeminiChatModel`） | `liteflow-agent-gemini/` |
| `src/providers/ollama.rs` | Ollama 本地模型 | ⏭️ 未复制（用 agentscope 原生 `OllamaChatModel`） | （待建薄包装 crate） |
| `src/providers/openai_codex.rs` | OpenAI Codex / Responses API | ✅ 完整复制（重构 AuthService 解耦 Config，feature-gated） | `liteflow-agent-provider-core/src/openai_codex.rs` |
| `src/providers/reliable.rs` | Fallback + retry 包装器 | ✅ 完整复制 | `liteflow-agent-provider-core/src/reliable.rs` |
| `src/providers/router.rs` | 多 provider 模型路由 | ✅ 完整复制 | `liteflow-agent-provider-core/src/router.rs` |
| `src/providers/health.rs` | Provider 健康追踪（断路器） | ✅ 完整复制（去 tracing 日志） | `liteflow-agent-provider-core/src/health.rs` |
| `src/providers/quota_adapter.rs` | Quota 解析适配 | ✅ 完整复制 | `liteflow-agent-provider-core/src/quota_adapter.rs` |
| `src/multimodal.rs` | 多模态图片处理 | ✅ 完整复制（内联 MultimodalConfig，feature-gated） | `liteflow-agent-provider-core/src/multimodal.rs` |
| `src/auth/` | OAuth token 管理（OpenAI/Gemini） | ✅ 完整复制（重构 AuthService::from_config 解耦 Config） | `liteflow-agent-provider-core/src/auth/`（7 文件：mod/anthropic_token/oauth_common/openai_oauth/gemini_oauth/profiles/secrets） |
| `src/security/secrets.rs` | 加密 secret 存储（ChaCha20-Poly1305） | ✅ 完整复制 | `liteflow-agent-provider-core/src/auth/secrets.rs` |

**整合统计**：**21 个 zeroclaw 源文件全部已实际纳入**（完整复制或精简），
覆盖 6 个 zeroclaw 独有平台 + provider-core 内核 + 全部基础设施
（backoff/health/quota_adapter/router/reliable/multimodal/util/proxy/runtime_options）
+ auth 子系统（mod/anthropic_token/oauth_common/openai_oauth/gemini_oauth/profiles/secrets）
+ codex（openai_codex.rs）。
4 个 agentscope 已有原生实现的平台（openai/anthropic/gemini/ollama）未复制 zeroclaw 代码，
直接复用 agentscope 的 `*ChatModel`（这是既定架构决策，避免重复实现）。

### Modifications

Each file derived from ZeroClaw carries a prominent header notice of the form:

```
// Copyright 2025 ZeroClaw Labs.
// Modified by LiteFlow-Rust contributors.
// Source: https://github.com/zeroclaw-labs/zeroclaw (Apache-2.0)
// SPDX-License-Identifier: Apache-2.0
```

Modifications include: adjusting import paths to the new module layout,
decoupling from ZeroClaw's `Config`/`AuthService` globals, and exposing
the providers through the LiteFlow-Rust `liteflow-agent` crate hierarchy.

### Trademark notice

"ZeroClaw" is a trademark of ZeroClaw Labs. LiteFlow-Rust is **not** an
official ZeroClaw product, is not affiliated with or endorsed by ZeroClaw
Labs, and does not use the ZeroClaw name in its crate names or product
identity. The ZeroClaw project is credited here solely for code attribution
as required by the Apache License 2.0.

## AgentScope-Rust

- **Project:** AgentScope-Rust (https://github.com/agentscope-ai/agentscope-rust)
- **Copyright:** © 2024-2026 the original author or authors.
- **License:** Apache-2.0
- **Used in:** `agentscope-core` is consumed as a path dependency by
  `liteflow-agent-core` and the provider adapter crates. The `Model` trait,
  `Msg`, `ToolSchema`, `GenerateOptions`, and the built-in `*ChatModel`
  implementations originate from AgentScope-Rust.

## LiteFlow (Java upstream)

- **Project:** LiteFlow (https://github.com/dromara/liteflow)
- **Copyright:** © Bryan.Zhang and LiteFlow contributors.
- **License:** Apache-2.0
- **Used in:** LiteFlow-Rust is a Rust reimplementation of LiteFlow's rule
  engine semantics. The Java project is the design reference; no Java source
  code is compiled into this project.

---

For the full text of each license, see the `LICENSE` file in this repository
(Apache-2.0) and the upstream project repositories.
