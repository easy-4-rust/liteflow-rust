# liteflow-rust S7 阶段：liteflow-agent（基于 agentscope-rust 的 ReAct Agent）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Java `liteflow-react-agent` 的 47 个 Agent 对象迁移到 `liteflow-agent`，基于 agentscope-rust 实现 ReAct Agent 编排，补齐 provider 真实网络契约、凭证生命周期和工具执行沙箱。

**Architecture:** `liteflow-agent/` 包含 `liteflow-agent-core`（ReAct/Hook/Session/Skill/Tool）+ 10 个 provider 子 crate（anthropic/openai/glm/copilot/bedrock/openrouter/telnyx/compatible/dashscope/gemini）+ `liteflow-agent-provider-core`（公共 Provider 基础设施）。

**Tech Stack:** agentscope-rust、reqwest、tokio、serde。

## Global Constraints

- Agent provider 通过 `Model`/Provider 契约接入，不把 OAuth、重试、限流和网络细节堆入 `liteflow-agent-core`。
- 工具执行生产环境必须使用进程/容器沙箱。
- provider 真实外部模型网络调用需可控测试环境验证。

---

### Task 1: Agent Core 迁移

- [x] **Step 1: ReAct 核心组件**

`ReActAgentComponent`、`ReActAgentContext`、`ReActLoggingHook`（文件名已按命名规则收口为 `re_act_*`）。

- [x] **Step 2: Session/Skill/Tool**

会话隔离、技能注册、工具执行框架。

### Task 2: Provider 适配

- [x] **Step 1: provider-core 公共基础设施**

`liteflow-agent-provider-core`：Model trait、Provider SPI、凭证管理。

- [x] **Step 2: 10 个 provider 子 crate**

anthropic/openai/glm/copilot/bedrock/openrouter/telnyx/compatible/dashscope/gemini。

- [ ] **Step 3: provider 真实网络契约验证**

OAuth/token、退避、限流、流式响应、工具调用和多模态契约。

- [ ] **Step 4: 凭证生命周期验证**

凭证刷新、速率限制、取消。

### Task 3: 工具执行沙箱

- [ ] **Step 1: 进程/容器沙箱**

`execute_shell_command` 的生产级隔离（当前为子进程超时强杀 + 输出上限 + 越界防护，非 OS 级沙箱）。

---

## 当前状态

**部分完成**。Agent core 和 12 个 provider 子 crate 均已创建（liteflow-agent 182 个 .rs 文件），基础框架已就位。剩余工作集中在 provider 真实网络验证、凭证生命周期和工具执行沙箱。

## 验证证据

| 检查项 | 当前状态 | 证据 |
|---|---|---|
| liteflow-agent 子 crate | 12 个 | provider-core + 10 个 provider + core |
| liteflow-agent .rs 文件数 | 182 | `find liteflow-agent -name "*.rs" | wc -l` |
| ReAct 核心 | ✅ 已收口 | 对象名称一致性检查 §3.3 |
| provider 真实网络 | 未验证 | 语义迁移对照表 §九 |
| 工具沙箱 | 进程级防护，非 OS 沙箱 | 语义迁移对照表 §九 |
