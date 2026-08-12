# liteflow-rust S8 阶段：Benchmark + Testcase-EL + 文档收尾

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 benchmark 性能对齐、testcase-el 测试体系建设（含 Java 3,739 个测试的逐项迁移账本）、全 workspace 100% 覆盖率目标、文档收尾和发布认证。

**Architecture:** `liteflow-benchmark/`（6 个子 crate）对齐 Java POM 声明的 benchmark。`liteflow-testcase-el/`（30+ 个子 crate）覆盖全部能力域的集成测试。覆盖率门禁要求全 workspace/all-features 的 region/function/line 均为 100%。

**Tech Stack:** criterion（benchmark）、cargo llvm-cov、tokio。

## Global Constraints

- benchmark 对齐数据规模、并发度、预热和统计方法，记录吞吐、P95/P99、内存。
- 测试对照无未裁决项，性能回归阈值可执行。
- 覆盖率禁止使用 `#[coverage(off)]`、忽略生产文件、删除分支或关闭 feature 提升数字。
- CI 门禁：`cargo test --workspace --all-features`、Clippy、rustdoc、格式、红线和文档审计。

---

### Task 1: Benchmark 体系

- [ ] **Step 1: liteflow-benchmark 6 个子 crate**

对齐 Java POM 声明的 8 个 benchmark 模块。

- [ ] **Step 2: 性能基线与回归阈值**

数据规模、并发度、预热和统计方法标准化。

### Task 2: Testcase-EL 测试体系

- [x] **Step 1: 30+ testcase 子 crate 已创建**

`liteflow-testcase-el/` 下已有 30 个子 crate，覆盖 nospring/vernal/script-*/rule-*/agent/builder/routechain 等。

- [ ] **Step 2: Java 3,739 个测试逐项迁移账本**

按能力域建立迁移账本：每项标记已迁移/替代/不适用。

- [ ] **Step 3: 29 个 testcase 模块 Java→Rust 用例对照**

缺失用例必须有原因。

### Task 3: 覆盖率闭环

- [ ] **Step 1: 全 workspace 100% 覆盖率**

当前基线：Region 78.36% / Function 77.29% / Line 78.75%（2026-07-29 workspace 基线）。

core 包最新覆盖率：Region 93.59% / Function 94.07% / Line 93.99%（2026-08-04）。

缺口：workspace 级仍需大量测试补齐。

### Task 4: 文档收尾

- [ ] **Step 1: 100% 覆盖率验收报告**

- [ ] **Step 2: 发布清单**

发布物不含本机绝对 path dependency；feature 组合和 MSRV 明确。

### Task 5: 生产认证

- [ ] **Step 1: 多节点、跨网络、TLS/ACL、滚动升级、故障注入和 24/72 小时长稳**

- [ ] **Step 2: 安全审计、许可证清单、回滚预案和数据兼容说明**

---

## 当前状态

**进行中**。testcase-el 30+ 子 crate 已创建，core 包覆盖率已达 93%+，但 workspace 级 100% 目标和 Java 测试迁移账本仍为重大缺口。

## 验证证据

| 检查项 | 当前状态 | 证据 |
|---|---|---|
| testcase-el 子 crate | 30 个 | `liteflow-testcase-el/` 下 30 个子 crate |
| testcase .rs 文件数 | 33 | `find liteflow-testcase-el -name "*.rs" | wc -l` |
| benchmark 子 crate | 已创建 | `liteflow-benchmark/` 存在，19 个 .rs 文件 |
| core 覆盖率 | Region 93.59% | 2026-08-04 迁移路线图 §10.2 |
| workspace 覆盖率 | Region 78.36% | 2026-07-29 基线 |
| 100% 目标 | 未达到 | 迁移路线图多次记录"100% 总门禁继续未通过" |
