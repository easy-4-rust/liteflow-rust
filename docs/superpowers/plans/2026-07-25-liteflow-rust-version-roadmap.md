# liteflow-rust 版本规划：v2.10 基线 → v2.16 语义超集 → 目标版本对齐

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 明确 liteflow-rust 的版本对齐策略：以 Java v2.10.0 模块边界建 workspace，保留 v2.16.0 语义超集，最终发布与 Java v2.16.x 功能完全对齐的 Rust 版本。

**Architecture:** 版本对齐分三阶段：(1) v2.10.0 模块边界 workspace 化；(2) v2.16.0 语义增量逐批补入；(3) 发布版本与 Java v2.16.x 功能完全对齐。

**Tech Stack:** Cargo workspace、语义化版本。

## Global Constraints

- 基线版本：现有代码已对齐 v2.16.0（功能超集）；用户指定 v2.10.0 基线 → 语义覆盖无冲突，以 v2.10 模块边界建 workspace，保留 2.16 语义。
- 发布版本必须通过全量测试、覆盖率门禁和红线扫描。
- 发布物不含本机绝对 path dependency。

---

## 1. 版本对齐策略

### 1.1 v2.10.0 基线（已完成）

以 Java v2.10.0 的模块边界定义 workspace 成员：
- `liteflow-core`（304 对象）
- `liteflow-el-builder`（18 对象）
- `liteflow-rule-plugin`（61 对象）
- `liteflow-script-plugin`（23 对象）
- `liteflow-spring` + starters（47 对象）→ `liteflow-vernal`
- `liteflow-react-agent`（47 对象）→ `liteflow-agent`

### 1.2 v2.16.0 语义增量（进行中）

v2.16.0 相对 v2.10.0 的关键增量：
- `execute2RespWithEL`（直接执行 EL，匿名链缓存）
- `ExecuteOption`（requestId/conversationId/autoConversationId/contextClass/contextBeans/eventListener）
- `FlowEvent` / `FlowEventListener` / `FlowEventPublisher`
- `DefaultContext`（ConcurrentHashMap、null 拒绝、getDataMap）
- Condition 级 bind（`THEN(...).bind(k,v[,override])`）
- Chain 级 bind（ChainBindWrapperCondition）
- `NodeIdUnIllegalException`（nodeId 变量命名规则校验）
- AND/OR 求值前按 isAccess 过滤
- `ScriptValidator.validate/validateWithEx`
- `SwitchCondition.getTargetList` / `NodeSwitchComponent.getTargetList`
- `ChainCacheLifeCycle`（Caffeine 缓存清理过期链）
- `ThreadPoolOperator` 自定义线程池
- `ParserHelper` 两阶段重构（parse → compile 分离）

**当前状态：** v2.16.0 语义增量已基本完成（详见《语义迁移对照表》§八）。

### 1.3 发布版本目标

- [ ] **Step 1: 全量测试通过**

`cargo test --workspace --all-features` 全绿。

- [ ] **Step 2: 覆盖率门禁通过**

全 workspace/all-features 的 region/function/line 均为 100%。

- [ ] **Step 3: 红线扫描通过**

0 compat.rs、0 生产 todo!/unimplemented!、0 lib.rs/mod.rs 公共类型。

- [ ] **Step 4: Clippy + rustdoc + 格式检查通过**

- [ ] **Step 5: 发布物不含 path dependency**

- [ ] **Step 6: MSRV 明确**

---

## 2. 里程碑对照

| 里程碑 | Java 对应 | Rust 状态 | 说明 |
|---|---|---|---|
| M0 基线与文档账本 | — | ✅ 已完成 | 四份权威文档已建立（现迁移至 superpowers） |
| M1 对象与名称闭环 | — | ✅ 已完成 | 500/500 均有唯一裁决 |
| M2 核心 API 差分 | — | 🔶 进行中 | CodeGraph 缺失，方法级审计待恢复 |
| M3 脚本集成闭环 | — | 🔶 进行中 | 引擎已就位，统一生命周期待补齐 |
| M4 规则插件 | — | 🔶 进行中 | 6 类已实现，SQL 多数据库/集群/长稳待验证 |
| M5 Vernal 宿主 | — | 🔶 进行中 | 对象已落位，Axum/Actix/path dependency 待完成 |
| M6 Agent | — | 🔶 进行中 | 框架已就位，provider 网络/凭证/沙箱待验证 |
| M7 测试与性能 | — | ⬜ 未开始 | Java 测试迁移账本、benchmark 基线 |
| M8 生产认证 | — | ⬜ 未开始 | 多节点/长稳/安全/发布 |
