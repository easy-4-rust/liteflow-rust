# liteflow-rust S0-S1 阶段：Workspace 化与 P0 纯搬迁拆分

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 liteflow-rust Cargo workspace 初始化，将 P0 纯搬迁模块（enums/exception/lifecycle/monitor/util）按一文件一对象规则拆分落地，建立 75 个基础测试的绿灯基线。

**Architecture:** 以 dromara/liteflow v2.16.0 为唯一 Java 事实源，在 `liteflow-core` crate 下按 Java 包同名目录建 Rust 模块结构。enums(11) / exception(基类枚举+59 具体异常=61) / lifecycle(5) / monitor(2) / util 模块逐文件拆分，每个 `.rs` 文件只对应一个 Java 对象。

**Tech Stack:** Rust edition 2021、Cargo workspace、serde/serde_json、tokio、dashmap、tracing。

## Global Constraints

- 目录/文件名 snake_case，文件内类型 PascalCase，方法 snake_case。
- Java 子包对应 Rust 同名子目录；多层嵌套只要求最后一级完全对齐。
- 每个 .rs 文件只对应一个 Java 对象；禁止 lib.rs/compat.rs 堆积对象。
- 参数命名与 Java 一致；从 Java 复制注释并中文化。
- 禁止 `todo!`/`unimplemented!`、`compat.rs`、生产 wildcard import。

---

### Task 1: S1.0 Workspace 化

- [x] **Step 1: 初始化 Cargo workspace**

创建根 `Cargo.toml` 定义 workspace 成员：`liteflow-core`。建立 `liteflow-core/src/` 目录结构。

- [x] **Step 2: 建立基础目录骨架**

按 Java 包结构建 `enums/`、`exception/`、`lifecycle/`、`monitor/`、`util/`、`flow/`、`core/`、`builder/`、`common/`、`context/`、`meta/`、`parser/`、`rule/`、`script/`、`slot/`、`spi/`、`thread/` 子目录。

### Task 2: S1.1 enums 拆分（11 文件）

- [x] **Step 1: 逐文件拆分 11 个枚举对象**

每个 Java 枚举对应独立 `.rs` 文件：`ChainExecuteModeEnum`、`CmpStepTypeEnum`、`ConditionTypeEnum`、`ExecuteableTypeEnum`、`FlowParserTypeEnum`、`LanguageTypeEnum`、`NodeEnum`、`NodeTypeEnum`、`ParseModeEnum`、`ScriptTypeEnum`、`SlotStateEnum`。

### Task 3: S1.2 lifecycle 拆分（5 文件）

- [x] **Step 1: 拆分 5 个生命周期对象**

`ChainBuildLifeCycle`、`ChainExecuteLifeCycle`、`FlowExecuteLifeCycle`、`NodeBuildLifeCycle`、`ScriptEngineInitLifeCycle`。

### Task 4: S1.3 monitor/util 拆分

- [x] **Step 1: monitor 2 个对象**

`MonitorBus`、`CompStatistics`。

- [x] **Step 2: util 模块工具类**

`ElRegexUtil`、`JsonUtil`、`SerialsUtil` 等工具对象。

### Task 5: S1.4 exception 拆分（61 文件）

- [x] **Step 1: 基类枚举 + 59 具体异常**

`LiteflowError` 枚举承载全部异常变体，59 个具体异常文件各对应一个 Java 异常类。

### Task 6: S1.5 推送与验证

- [x] **Step 1: 推送 dev 分支**

- [x] **Step 2: 整库 diff 验证一致**

---

## 验证证据

| 检查项 | 结果 | 证据 |
|---|---|---|
| git log S1 提交 | 2026-07-25，14 个提交 | `55db160`-`0cbde9b`，workspace 化 + enums/lifecycle/monitor/util/exception 拆分 |
| plan.md S1 标记 | ✅ 已标记完成 | "S1 P0 纯搬迁拆分 ✅（2026-07-25 完成并推送 dev）" |
| 测试基线 | 75 测试保持绿 | plan.md 记载 |
| 一文件一对象 | 红线扫描 0 错误 | 布局审计 `scanned=890 errors=0` |

## 完成日期

2026-07-25
