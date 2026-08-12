# liteflow-rust S4 阶段：liteflow-derive 过程宏

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `liteflow-derive` proc-macro crate，提供 `#[liteflow_component]`、`#[liteflow_method]`、`#[liteflow_cmp_define]`、`#[liteflow_retry]`、`#[fallback_cmp]`、`#[liteflow_fact]` 等声明式组件注解的 Rust 编译期等价。

**Architecture:** `liteflow-derive` 作为独立 proc-macro crate，在编译期生成 FlowBus 注册入口、静态分派代理、方法元数据和重试/降级配置。替代 Java 运行期反射 + ByteBuddy 代理。

**Tech Stack:** Rust proc-macro、syn、quote、proc-macro2。

## Global Constraints

- 过程宏生成的代码必须符合项目 snake_case/PascalCase 命名规范。
- 生成的注册入口必须与 Java `@LiteflowComponent` 的 FlowBus 注册语义一致。
- `#[liteflow_cmp_define]` 的方法级 `value/node_id/node_name/node_type` 必须按 Java 规则生成多 `DeclWarpBean`。

---

### Task 1: 核心注解宏

- [x] **Step 1: `#[liteflow_component]`**

属性宏，生成 FlowBus 显式注册入口。

- [x] **Step 2: `#[liteflow_method]`**

由外层 `cmp_define` 宏消费；支持 Java 对等 `value/node_id/node_name/node_type`。

- [x] **Step 3: `#[liteflow_cmp_define]`**

编译期生成 `DeclComponent` 静态分派与注册入口。类级类型覆盖、方法级多 nodeId 分组、主方法校验、全部生命周期角色。

- [x] **Step 4: `#[liteflow_retry]`**

生成 `retry_count`/`is_retry_for`，保留原始异常类型过滤。

- [x] **Step 5: `#[fallback_cmp]`**

生成 id/type 元数据与 `register_fallback`；按 COMMON/BOOLEAN/SWITCH/FOR/ITERATOR 位置选择。

- [x] **Step 6: `#[liteflow_fact]`**

从 Slot bean 按名称注入 `Arc<T>`。

### Task 2: 声明式组件验证

- [x] **Step 1: trybuild 编译契约**

7 个 trybuild 编译夹具覆盖合法与非法声明。

- [ ] **Step 2: Java declare testcase 自动差分**

多 nodeId、全部方法角色和非法声明进入真实测试；逐项运行 Java declare testcase 并建立自动差分账本。

---

## 当前状态

**进行中**。核心注解宏均已实现（✅），trybuild 编译契约通过。剩余工作是 Java declare testcase 自动差分验收。

## 验证证据

| 检查项 | 当前状态 | 证据 |
|---|---|---|
| liteflow-derive crate | 已创建 | `liteflow-derive/` 存在，22 个 .rs 文件 |
| 6 个注解宏 | 均已实现 | 对象级对照表 §1 annotation（6 个 ✅/🔶） |
| trybuild 契约 | 7 个通过 | 迁移路线图 §2.3 记载 |
| 声明式组件执行路径 | 已接入 | 访问/结束/错误继续/前后置/成功/错误/回滚均进入真实 Node 生命周期 |
