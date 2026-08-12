# liteflow-rust S5 阶段：Script-Plugin 全量 + Rule-Plugin 订阅/轮询分层补齐

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 `liteflow-script-plugin` 全量脚本引擎适配（8 种语言），补齐 `liteflow-rule-plugin` 六类规则源的订阅/轮询分层、TLS/ACL、集群故障和长稳验证。

**Architecture:** 脚本引擎在 `liteflow-script-plugin/` 下各子 crate 独立实现，统一 `ScriptExecutor` trait 契约。规则插件在 `liteflow-rule-plugin/` 下各子 crate 独立实现，统一 `RuleSource` trait + watcher/polling/Monitor 分层。

**Tech Stack:** qlexpress 0.1.0-alpha.1、mlua 0.12、pyo3 0.29、boa_engine 0.21、groovy（受控 Rhai 适配）、Kotlin（受控 Rust 解释层）、Aviator（受控 Rust 解释层）、reqwest、etcd-client、redis、zookeeper-client 0.11.2、nacos-sdk。

## Global Constraints

- 脚本引擎只承诺受控子集，不宣称完整 JVM 语言兼容。
- 规则插件必须使用 `tokio-util::CancellationToken` 管理 watcher/polling 生命周期。
- 外部服务测试需本机真实服务或 fixture，TLS/ACL/集群/长稳为独立验收维度。

---

### Task 1: 脚本引擎全量适配

- [x] **Step 1: QLExpress 脚本引擎**

`liteflow-script-qlexpress`：使用 crates.io `qlexpress 0.1.0-alpha.1` 真实 lexer/parser/compiler/QVM。

- [x] **Step 2: Lua 脚本引擎**

`liteflow-script-lua`：mlua 0.12 真实 Lua 执行。

- [x] **Step 3: JavaScript 脚本引擎**

`liteflow-script-javascript`：boa_engine 0.21 真实 JavaScript 执行。

- [x] **Step 4: Python 脚本引擎**

`liteflow-script-python`：pyo3 0.29 CPython 嵌入。

- [x] **Step 5: GraalJS 脚本引擎**

`liteflow-script-graaljs`：GraalJS 适配。

- [ ] **Step 6: 脚本引擎统一生命周期补齐**

每种引擎的 `load/execute/remove/reload/validate` 生命周期、隔离、超时、资源上限和 ScriptBean/context bean 映射。

- [ ] **Step 7: Kotlin remove 后缓存/节点 ID/源码/重载路由语义补齐**

### Task 2: 规则插件订阅/轮询分层补齐

- [x] **Step 1: Apollo 规则插件**

`liteflow-rule-apollo`：HTTP 协议 + 指纹 watcher。

- [x] **Step 2: Nacos 规则插件**

`liteflow-rule-nacos`：原生监听。

- [x] **Step 3: ZooKeeper 规则插件**

`liteflow-rule-zk`：zookeeper-client 0.11.2 原生 watch。

- [x] **Step 4: Etcd 规则插件**

`liteflow-rule-etcd`：etcd-client 原生 watch。

- [x] **Step 5: Redis 规则插件**

`liteflow-rule-redis`：redis crate 订阅/轮询。

- [x] **Step 6: SQL 规则插件**

`liteflow-rule-sql`：rusqlite 0.32.1 SQLite 单实现。

- [ ] **Step 7: watcher/monitor 取消与优雅关闭**

所有 watcher/polling/Monitor 使用 CancellationToken，关闭时等待任务和连接退出。

- [ ] **Step 8: Apollo 异步客户端与真实集群**

协议/鉴权/灰度/故障测试。

- [ ] **Step 9: SQL 三数据库**

SQLite/MySQL/PostgreSQL 同契约（sqlx 或 rbdc 抽象）。

- [ ] **Step 10: Nacos/ZK/Etcd/Redis 真实服务验证**

TLS/ACL、集群故障、断线重连、重复事件、长稳。

---

## 当前状态

**部分完成**。8 个脚本引擎子 crate 和 6 个规则插件子 crate 均已创建并有基础实现。剩余工作集中在统一生命周期补齐、Kotlin remove/reload、SQL 多数据库、外部服务集群/鉴权/长稳验证。

## 验证证据

| 检查项 | 当前状态 | 证据 |
|---|---|---|
| 脚本引擎子 crate | 8/8 已创建 | `liteflow-script-plugin/` 下 8 个子 crate |
| 规则插件子 crate | 6/6 已创建 | `liteflow-rule-plugin/` 下 6 个子 crate |
| 脚本引擎 .rs 文件数 | 36 | `find liteflow-script-plugin -name "*.rs" | wc -l` |
| 规则插件 .rs 文件数 | 137 | `find liteflow-rule-plugin -name "*.rs" | wc -l` |
| testcase 覆盖 | 多个 testcase-el 子 crate | script-*-vernal、rule-*-vernal 等 |
