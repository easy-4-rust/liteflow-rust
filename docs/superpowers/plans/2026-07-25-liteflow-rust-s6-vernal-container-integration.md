# liteflow-rust S6 阶段：liteflow-vernal 容器集成 + Axum/Actix Starter

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 `liteflow-vernal` crate 的容器集成，将 Java Spring/Boot 3/Boot 4/Solon 的 47 个宿主对象逐个裁决并迁移到 Vernal 容器 + Axum/Actix 适配，消除跨仓库 path dependency。

**Architecture:** `liteflow-vernal` 承载 Java `liteflow-spring` + `liteflow-spring-boot-starter` + `liteflow-spring-boot4-starter` + `liteflow-solon-plugin` 的全部语义。配置通过 `ConfigurationProperties` 统一绑定，生命周期通过 Vernal 容器 SPI 管理，Web 层通过 Axum（Spring Boot 等价）和 Actix（Quarkus 等价）适配。

**Tech Stack:** Vernal 容器框架、Axum、Actix-web、serde、tokio。

## Global Constraints

- Spring→Vernal 8 个名称映射已登记为正式批准例外，不回改。
- Boot 3/Boot 4/Solon 同名简单类依靠包路径区分。
- 发布前必须消除跨仓库 path dependency（vernal-framework 等）。

---

### Task 1: 宿主对象逐个裁决

- [x] **Step 1: Spring 容器 SPI（6 个）**

`VernalAware`、`VernalCmpAroundAspect`、`VernalContextCmpInit`、`VernalDeclComponentParser`、`VernalLiteflowComponentSupport`、`VernalPathContentParser`。

- [x] **Step 2: Spring 顶层容器对象（2 个）**

`VernalComponentScanner`、`VernalDeclBeanDefinition`。

- [ ] **Step 3: process/* 12 个 Java 对象**

核实 12 个 `process/*` 对象是否逐文件落位。

- [x] **Step 4: Spring Boot 3 starter（5 个）**

`LiteflowProperty`、`LiteflowMonitorProperty`、`LiteflowPropertyAutoConfiguration`、`LiteflowMainAutoConfiguration`、`LiteflowExecutorInit`。

- [x] **Step 5: Spring Boot 4 starter（5 个）**

独立 `springboot4/` 包，非 Boot 3 type alias。

- [x] **Step 6: Solon 插件（15 个）**

Java v2.16.0 的 15 个生产对象均有独立 Rust 实现证据。

### Task 2: Axum/Actix 适配

- [ ] **Step 1: Axum 请求作用域、优雅停机、配置刷新**

- [ ] **Step 2: Actix 请求作用域、优雅停机、配置刷新**

- [ ] **Step 3: 多 ApplicationContext 隔离**

### Task 3: 发布依赖收口

- [ ] **Step 1: 消除跨仓库 path dependency**

`vernal-context` 与 `vernal-context-support` 改为 crates.io/git 锁定版本，或并入同一发布工作区。

---

## 当前状态

**进行中**。47 个 Java 宿主对象中大部分已有独立 Rust 实现（liteflow-vernal 76 个 .rs 文件），但 Axum/Actix 请求作用域、优雅停机、多上下文隔离和发布依赖收口仍需完成。

## 验证证据

| 检查项 | 当前状态 | 证据 |
|---|---|---|
| liteflow-vernal .rs 文件数 | 76 | `find liteflow-vernal -name "*.rs" | wc -l` |
| Spring SPI 6 个 | ✅ 已裁决 | 对象名称一致性检查 §3.2 |
| Boot 3 starter 5 个 | ✅ 已落位 | 语义迁移对照表 §九 |
| Boot 4 starter 5 个 | ✅ 已落位 | 语义迁移对照表 §九 |
| Solon 15 个 | ✅ 已裁决 | 语义迁移对照表 §九 |
| Axum/Actix 适配 | 进行中 | 待验证 |
| path dependency | 未消除 | 迁移路线图 §5 M5 退出条件 |
