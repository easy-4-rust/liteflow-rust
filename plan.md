# liteflow-rust 全量迁移工程 plan

## 目标
以 dromara/liteflow 为蓝本，实现能力完全对齐的 liteflow-rust workspace：
liteflow-core / liteflow-derive / liteflow-rule-plugin / liteflow-script-plugin /
liteflow-vernal / liteflow-el-builder / liteflow-agent。

## 硬性规范（用户指定）
1. 目录/文件名 snake_case（context/、dao/），文件内类型 PascalCase，方法 snake_case
2. Java 子包 → Rust 同名子目录；多层嵌套只要求最后一级完全对齐
3. 参数命名与 Java 一致；从 Java 复制注释并中文化（对象/方法/代码段注释，标注 Java 对应文件与方法）
4. jackson→serde；Spring Boot→axum；Quarkus→actix；Spring 容器→自研 vernal；Agent→agentscope-rust
5. 每个 .rs 文件只对应一个 Java 对象；禁止 lib.rs/compat.rs 堆积对象；禁止 compat.rs 转发式引用
6. annotation/ 独立为 liteflow-derive（过程宏 crate）
7. 模板引擎对应速记：Tera≈FreeMarker、Handlebars≈Velocity、Askama≈编译期 JSP/Thymeleaf、maud≈Twirl/JSX
8. 迁移原则：功能语义对齐，实现方式 Rust 化

## 阶段规划
- S0 盘点（本阶段）：当前完成度评估，对照 v2.10.0 基线与 304 类对象级清单
- S1 P0 纯搬迁拆分：enums(11 文件) / exception(61 文件=基类枚举+59 具体异常) / lifecycle(5) / monitor(2) / util → 一文件一对象，75 测试保持绿 ✅（2026-07-25 完成并推送 dev：S1.0 workspace 化 → S1.1 enums → S1.2 lifecycle → S1.3 monitor/util → S1.4 exception → S1.5 推送+整库 diff 验证一致）
- S2 P1 主干补缺 ✅（2026-07-25 完成，101 测试全绿 + all-features 编译通过）：
  - S2-A flow/executor：NodeExecutor 重试主干 + DefaultNodeExecutor + NodeExecutorHelper + flow/parallel 三件套
  - S2-B spi 体系 17 类（5 接口+5 holder+5 local+SpiPriority+SpiFactoryCleaner）+ flow/id RequestIdGenerator 3 类
  - S2-C 36 个缺失异常变体挂接 LiteflowError
  - S2-D condition 补齐：ConditionKey + 13 类方法级语义比对补缺
- S3 P2 builder/el/operator 34 类拆分 + liteflow-el-builder（ELBus 链式 API）
- S4 liteflow-derive（annotation/ 过程宏：@LiteflowComponent/@LiteflowMethod/@LiteflowRetry/@FallbackCmp 语义）
- S5 liteflow-script-plugin 全量（lua/graaljs→boa/js→quickjs/python→pyo3/groovy→rhai?/qlexpress→自研）与 rule-plugin 订阅/轮询分层补齐
- S6 liteflow-vernal：基于 easy-4-rust/vernal 的容器集成 + axum(Spring Boot)/actix(Quarkus) starter
- S7 liteflow-agent：基于 agentscope-rust 的 ReAct Agent 迁移
- S8 benchmark + testcase-el 测试体系 + 文档收尾

## 风险
- 基线版本：现有代码已对齐 v2.16.0（功能超集）；用户指定 v2.10.0 基线 → 语义覆盖无冲突，以 v2.10 模块边界建 workspace，保留 2.16 语义
- 沙箱环境 $HOME//tmp 周期性清空：工具链恢复脚本 /mnt/agents/restore-rust.sh
