# liteflow-rust S2 阶段：P1 主干补缺

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补齐 P1 主干核心能力：FlowExecutor/NodeExecutor 重试主干、SPI 体系 17 类、36 个缺失异常变体挂接、Condition 补齐，达成 101 测试全绿 + all-features 编译通过。

**Architecture:** 在 liteflow-core 已有 S1 骨架上，按 Java v2.16.0 源码逐对象补缺。SPI 体系包含 5 接口 + 5 holder + 5 local + SpiPriority + SpiFactoryCleaner；flow/executor 包含 NodeExecutor 重试主干 + DefaultNodeExecutor + NodeExecutorHelper；flow/parallel 三件套。

**Tech Stack:** Rust edition 2021、Cargo workspace、serde/serde_json、tokio、dashmap、tracing。

## Global Constraints

- 同 S1 约束：snake_case 文件名、一文件一对象、中文注释、无 todo!/unimplemented!。
- 每个新增对象必须有对应 Java FQN 注释。
- 测试放在源码内 `#[cfg(test)]` 或外置 tests/ 目录。

---

### Task 1: S2-A flow/executor 补缺

- [x] **Step 1: NodeExecutor 重试主干**

`NodeExecutor` trait + `DefaultNodeExecutor` + `NodeExecutorHelper`：重试循环、异常捕获、回滚逆序。

- [x] **Step 2: flow/parallel 三件套**

`ParallelStrategyHelper` + 四类并行执行器（AllOf/AnyOf/PercentageOf/Specify）。

### Task 2: S2-B SPI 体系 17 类

- [x] **Step 1: 5 接口 + 5 holder + 5 local**

`ContextAware`、`CmpAroundAspect`、`ContextCmpInit`、`DeclComponentParser`、`PathContentParser` 各有接口/Holder/Local 三件套。

- [x] **Step 2: SpiPriority + SpiFactoryCleaner**

SPI 优先级排序与工厂清理。

- [x] **Step 3: flow/id RequestIdGenerator 3 类**

`RequestIdGenerator` trait + 默认实现 + Holder。

### Task 3: S2-C 36 个缺失异常变体

- [x] **Step 1: 挂接 LiteflowError 枚举变体**

36 个缺失异常变体逐一挂接到 `LiteflowError` 枚举，确保每个 Java 异常类有对应 Rust 变体。

### Task 4: S2-D Condition 补齐

- [x] **Step 1: ConditionKey + 13 类方法级语义比对补缺**

`ConditionKey` + 13 个 Condition 类的方法级语义补缺。

### Task 5: 验证

- [x] **Step 1: 101 测试全绿**

- [x] **Step 2: all-features 编译通过**

---

## 验证证据

| 检查项 | 结果 | 证据 |
|---|---|---|
| git log S2 提交 | 2026-07-25，8 个提交 | `c439b62`-`7718d5b`，batch1+batch2 共 6 轮推送 |
| plan.md S2 标记 | ✅ 已标记完成 | "S2 P1 主干补缺 ✅（2026-07-25 完成，101 测试全绿 + all-features 编译通过）" |
| 测试数量 | 101 测试全绿 | plan.md 记载 |
| 编译 | all-features 通过 | plan.md 记载 |

## 完成日期

2026-07-25
