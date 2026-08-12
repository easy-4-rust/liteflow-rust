# liteflow-rust 已完成增强：Script 引擎 / Agent Provider / Rule-ZK / AOP

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 S1/S2 基础上完成多项已完成的增量增强：Aviator/Groovy/Kotlin 脚本引擎适配、Agent provider 适配器、ZooKeeper 0.11.2 迁移、AOP aspect-core 集成、连接池、API 对等性测试。

**Architecture:** 各增强项独立推进，均在已有 workspace 结构内增量实现。脚本引擎在 `liteflow-script-plugin` 下各子 crate 落地；Agent provider 在 `liteflow-agent` 下各子 crate 落地；rule-zk 在 `liteflow-rule-plugin/liteflow-rule-zk` 迁移。

**Tech Stack:** Rust edition 2021、mlua、pyo3、boa_engine、qlexpress、zookeeper-client 0.11.2、aspect-core/aspect-std、reqwest、tokio。

## Global Constraints

- 同 S1/S2 约束。
- 脚本引擎只承诺受控子集，不宣称完整 JVM 语言兼容。
- 外部服务测试需本机真实服务或 fixture，不标为生产完成。

---

### Task 1: Aviator 脚本引擎（2026-07-27）

- [x] **Step 1: 实现 Aviator 脚本 Java 基线语法支持**

`liteflow-script-aviator` crate：覆盖 Java 基线的 use/DateUtil/println/setData。

### Task 2: Groovy 脚本引擎（2026-07-27）

- [x] **Step 1: 实现 Groovy 脚本 LiteFlow 绑定语义的 Rust 受控执行器**

`liteflow-script-groovy` crate：覆盖 def/基础类型、DefaultContext、`_meta.cmpData`、println、if/else、FOR/ITERATOR 循环元数据。

### Task 3: Kotlin 脚本引擎（2026-07-27）

- [x] **Step 1: 添加 Kotlin 脚本支持功能**

`liteflow-script-kotlin` crate：对照 Java testcase 覆盖 val/var、显式基础类型、表达式/块函数、块级 if/else、bindings DefaultContext、ScriptBean、普通/Boolean/Switch/For 与 WHILE BREAK。

### Task 4: Agent Provider 适配（2026-07-27）

- [x] **Step 1: 新增 SQL 读取基类和轮询任务及 Provider 适配器**

`liteflow-agent-provider-core` + 各 provider 子 crate（anthropic/openai/glm/copilot/bedrock/openrouter/telnyx/compatible/dashscope/gemini）。

### Task 5: Rule-ZK 迁移（2026-08-01）

- [x] **Step 1: 迁移 zookeeper 0.8 到 zookeeper-client 0.11.2**

`liteflow-rule-zk` 依赖升级，适配新 API。

### Task 6: AOP 集成（2026-08-01）

- [x] **Step 1: 添加 aspect-core 和 aspect-std 依赖包**

AOP 切面框架集成，`ICmpAroundAspect` 与 aspect-core 桥接。

### Task 7: 连接池与依赖更新（2026-07-30）

- [x] **Step 1: 添加连接关闭过滤器和外部依赖库支持**

连接池管理、外部依赖版本更新。

### Task 8: FlowExecutor/Parser 重构（2026-07-28）

- [x] **Step 1: 实现 FlowExecutor 初始化与规则解析功能**

`FlowExecutor::init` 真实执行组件 SPI、ID、Parser、缓存、启动钩子。

- [x] **Step 2: 重构解析器实现将链定义和规则计划移至辅助模块**

`ParserHelper` + `RuleDefinitionPlan` 两阶段重构。

- [x] **Step 3: 实现节点实例 ID 管理 SPI 和服务启动恢复功能**

`NodeInstanceId` SPI + 文件默认实现 + Holder。

- [x] **Step 4: 重构条件执行与并行策略实现**

Condition 执行逻辑重构、并行策略统一。

### Task 9: API 对等性测试（2026-08-05）

- [x] **Step 1: 添加全面的 API 对等性测试覆盖配置、步骤和集合组件**

外置测试文件覆盖 FlowExecutor、CmpStep、Slot、LiteflowResponse 等核心 API。

---

## 验证证据

| 检查项 | 结果 | 证据 |
|---|---|---|
| git log 时间线 | 2026-07-27 ~ 2026-08-05 | `ae408fd`(aviator) → `4e0ca95`(API 对等性测试)，共 20+ 提交 |
| 脚本引擎 | aviator/groovy/kotlin 均有独立 crate | `liteflow-script-plugin/` 下 8 个子 crate |
| Agent provider | 12 个子 crate | `liteflow-agent/` 下含 provider-core + 10 个 provider |
| rule-zk | zookeeper-client 0.11.2 | `cdcff5d` 提交 |
| AOP | aspect-core/aspect-std 集成 | `ac4f8ec` 提交 |
| API 对等性测试 | 2026-08-05 提交 | `4e0ca95` |

## 完成日期

2026-07-27 ~ 2026-08-05（分批完成）
