# liteflow-rust S3 阶段：P2 Builder/EL/Operator 34 类拆分 + liteflow-el-builder

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `liteflow-core/src/builder/el/operator/` 下 34 个 Java Operator 对象逐文件拆分为独立 Rust 对象，同时完成 `liteflow-el-builder` crate 的 ELBus 链式 API，使代码式组装 EL 表达式成为可能。

**Architecture:** 34 个 Operator 在 `liteflow-core/src/builder/el/operator/` 下各建独立 `.rs` 文件；`liteflow-el-builder` 作为独立 crate 提供 ELBus + 18 个包装器，支持 Java 完整语句及可直接解析的运行时表达式。

**Tech Stack:** Rust edition 2021、qlexpress 0.1.0-alpha.1、serde。

## Global Constraints

- 同 S1/S2 约束：snake_case、一文件一对象、中文注释、无 todo!。
- Operator 对象必须与 Java v2.16.0 源码逐方法对齐。
- EL Builder API 必须覆盖 Java ELBus 的全部链式方法。

---

### Task 1: Operator 34 类拆分

- [x] **Step 1: 已完成的 Operator（28/34）**

以下 Operator 已完成独立拆分：AndOperator、AnyOperator、BindOperator、BreakOperator、CatchOperator、DataOperator、DefaultOperator、DoOperator、ElifOperator、ElseOperator、FinallyOperator、ForOperator、IdOperator、IfOperator、IgnoreErrorOperator、IteratorOperator、MaxWaitMillisecondsOperator、MaxWaitSecondsOperator、MaxWaitTimeOperator、MustOperator、NodeOperator、NotOperator、OrOperator、ParallelOperator、PercentageOperator、PreOperator、RetryOperator、SwitchOperator、TagOperator、ThenOperator、ThreadPoolOperator、ToOperator、WhenOperator、WhileOperator、BaseOperator、OperatorHelper。

- [ ] **Step 2: 待确认的 Operator 拆分状态**

核实全部 34+2（base）个 Operator 是否均已独立落位。当前对象表显示 36 个独立 Rust 对象，状态均为 ✅。

### Task 2: liteflow-el-builder ELBus 链式 API

- [x] **Step 1: ELBus + 18 个包装器**

`liteflow-el-builder` crate 已建，包含 ELBus 链式 API 和 18 个一文件一对象包装器。

- [ ] **Step 2: 覆盖 Java ELBus 全部方法**

核实 ELBus 是否覆盖 Java `ELBus` 类的全部公共方法。

### Task 3: 验证

- [ ] **Step 1: Operator 解析测试全覆盖**

每个 Operator 的 parse/convert 语义均有测试。

- [ ] **Step 2: EL Builder 运行时表达式测试**

EL Builder 组装的表达式可被 QLExpress 正确解析和执行。

---

## 当前状态

**进行中**。对象表显示 36 个 Operator 已独立落位（✅），但 EL Builder 的完整方法覆盖和端到端测试仍需核实。

## 验证证据

| 检查项 | 当前状态 | 证据 |
|---|---|---|
| Operator 独立文件 | 36/36 ✅ | 对象级对照表 §3 builder |
| EL Builder crate | 已创建 | `liteflow-el-builder/` 存在，31 个 .rs 文件 |
| liteflow-el-builder 文件数 | 31 | `find liteflow-el-builder/src -name "*.rs" | wc -l` |
