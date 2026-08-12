# liteflow-rust Superpowers 文档体系

本目录是 liteflow-rust 项目的规格驱动开发（SDD）文档中心，遵循 dromara/liteflow Java 参考项目的 `docs/superpowers` 命名与格式规范。

## 目录结构

```
docs/superpowers/
├── README.md                    # 本文件
├── plans/                       # 实施计划（日期前缀 + slug）
│   ├── 2026-07-25-liteflow-rust-s0-s1-workspace-and-p0-migration.md
│   ├── 2026-07-25-liteflow-rust-s2-p1-core-backfill.md
│   ├── 2026-07-27-liteflow-rust-script-agent-rule-zk-enhancements.md
│   ├── 2026-07-25-liteflow-rust-s3-el-builder-operator-split.md
│   ├── 2026-07-25-liteflow-rust-s4-derive-proc-macro.md
│   ├── 2026-07-25-liteflow-rust-s5-script-rule-plugin-full.md
│   ├── 2026-07-25-liteflow-rust-s6-vernal-container-integration.md
│   ├── 2026-07-25-liteflow-rust-s7-agent-react-migration.md
│   ├── 2026-07-25-liteflow-rust-s8-benchmark-testcase-docs.md
│   └── 2026-07-25-liteflow-rust-version-roadmap.md
└── specs/                       # 设计规范（日期前缀 + -design 后缀）
    ├── 2026-07-29-liteflow-rust-migration-roadmap-design.md
    ├── 2026-07-29-object-name-consistency-design.md
    ├── 2026-07-29-object-level-comparison-design.md
    └── 2026-07-29-semantic-migration-comparison-design.md
## 命名规范

- **计划文件**：`{YYYY-MM-DD}-{slug}.md`，slug 使用小写连字符。
- **规范文件**：`{YYYY-MM-DD}-{slug}-design.md`，以 `-design` 后缀区分。
- **日期**：使用 git log 中的真实提交日期或文档首次创建日期。
- **语言**：中文撰写，`For agentic workers` 引用行保留英文原样。

## 计划文件索引

| 文件 | 日期 | 阶段 | 状态 |
|---|---|---|---|
| S0-S1 Workspace 化与 P0 纯搬迁 | 2026-07-25 | S0-S1 | 已完成 |
| S2 P1 主干补缺 | 2026-07-25 | S2 | 已完成 |
| Script/Agent/Rule-ZK 增强 | 2026-07-27~08-05 | 增量 | 已完成 |
| S3 EL Builder/Operator 拆分 | 2026-07-25 | S3 | 进行中 |
| S4 liteflow-derive 过程宏 | 2026-07-25 | S4 | 进行中 |
| S5 Script-Plugin/Rule-Plugin 全量 | 2026-07-25 | S5 | 部分完成 |
| S6 Vernal 容器集成 | 2026-07-25 | S6 | 进行中 |
| S7 Agent ReAct 迁移 | 2026-07-25 | S7 | 部分完成 |
| S8 Benchmark/Testcase/文档收尾 | 2026-07-25 | S8 | 进行中 |
| 版本规划 | 2026-07-25 | 全局 | 活跃 |

## 规范文件索引

| 文件 | 日期 | 内容 |
|---|---|---|
| 迁移路线图规范 | 2026-07-29 | 目标/完成定义/基线/覆盖率/阶段计划/工程红线 |
| 对象命名与落位规范 | 2026-07-29 | 检查规则/基线规模/例外清单/裁决结论 |
| 对象级对照表规范 | 2026-07-29 | 500 个 Java→Rust 对象对照（含附录） |
| 语义迁移对照规范 | 2026-07-29 | 功能语义迁移对照（12 个能力域） |

## 与旧文档的关系

原 `docs/` 下的 4 个旧文档已完整迁移至 `docs/superpowers/specs/` 并删除：

| 旧文件 | 新文件 | 迁移方式 |
|---|---|---|
| `docs/迁移路线图.md` | `specs/2026-07-29-liteflow-rust-migration-roadmap-design.md` | 内容重组为规范格式 |
| `docs/对象名称一致性检查.md` | `specs/2026-07-29-object-name-consistency-design.md` | 内容重组为规范格式 |
| `docs/对象级对照表.md` | `specs/2026-07-29-object-level-comparison-design.md` | 全量内容保留（含附录） |
| `docs/语义迁移对照表.md` | `specs/2026-07-29-semantic-migration-comparison-design.md` | 全量内容保留（12 个能力域） |

`plan.md`（根目录）保留为精简版入口，链接到 superpowers 体系。

## 参考项目

本体系遵循 dromara/liteflow Java 项目的 `docs/superpowers` 规范：
- `docs/superpowers/plans/`：日期前缀计划文件
- `docs/superpowers/specs/`：日期前缀设计文件（`-design` 后缀）
