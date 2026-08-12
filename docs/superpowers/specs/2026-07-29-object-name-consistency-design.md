# liteflow-rust 对象命名与落位规范

日期：2026-07-29（2026-08-04 更新）
状态：活跃
落点：liteflow-rust 全 workspace 生产 `src/**/*.rs` 文件

---

## 1. 检查规则

1. Java 顶级类、接口、枚举、record 各对应一个真实 `.rs` 文件。
2. Java 类名转换为 snake_case 文件名；Rust 主类型保留 PascalCase 对象身份。
3. Java 子包对应 Rust 同名目录；多层嵌套至少对齐最后一级。
4. Java 同简单类名依靠包路径区分，例如 Boot 3、Boot 4、Solon 的
   `LiteflowProperty` 不是重复对象。
5. `mod.rs`/`lib.rs` 只声明和重导出；内部类、伴随 Builder、类型别名可以与主对象
   同文件，但必须在对象表中说明。
6. Java 生态专有对象不能直接删除：必须标成"Java 专有 + Rust 等价语义入口"或
   "尚未迁移"，并有测试或边界说明。

## 2. 基线规模

| Java 模块 | Java 对象 | 直接 basename 命中 | 未直接命中 | basename 多候选 |
|---|---:|---:|---:|---:|
| `liteflow-core` | 304 | 304 | 0 | 0 |
| `liteflow-el-builder` | 18 | 18 | 0 | 0 |
| `liteflow-rule-plugin` | 61 | 61 | 0 | 0 |
| `liteflow-script-plugin` | 23 | 8 | 15 | 0 |
| `liteflow-spring` | 22 | 14 | 8 | 0 |
| `liteflow-spring-boot-starter` | 5 | 5 | 0 | 5 |
| `liteflow-spring-boot4-starter` | 5 | 5 | 0 | 5 |
| `liteflow-solon-plugin` | 15 | 15 | 0 | 3 |
| `liteflow-react-agent` | 47 | 47 | 0 | 0 |
| **合计** | **500** | **477** | **23** | **13** |

"多候选"不是缺陷：Boot 3、Boot 4、Solon 在不同包中声明同名对象。加入 Java 包路径
后可以唯一定位。

2026-08-04 更新：直接同名由 472 升至 **477**（规则插件 2 项与 ReAct 3 项按命名
规则收口）；剩余 23 项未直接同名中，8 项 Spring→Vernal 已登记为**正式批准例外**
（第 3.2 节），15 项 JVM 专有脚本对象已完成**逐项裁决**并登记 Rust 等价入口
（第 4 节）。因此当前结论是：**500/500 均有唯一对象裁决，名称差异为 0 或全部为
经批准的显式例外**。

## 3. 非 JVM 名称差异

### 3.1 规则插件（2）——已收口

| Java 对象 | 收口后 Rust | 结论 |
|---|---|---|
| `com.yomahub.liteflow.parser.nacos.util.NacosParserHelper` | `parser/nacos/util/nacos_parser_helper.rs` / `NacosParserHelper` | 2026-08-04 文件与类型已按命名规则收口（`Parser` 不再被改为 `Parse`），crate 全特性编译通过 |
| `com.yomahub.liteflow.parser.sql.exception.ELSQLException` | `parser/sql/exception/elsql_exception.rs` / `ELSQLException` | 2026-08-04 按"连续大写缩写视为一个词"的项目缩写规则收口为 `elsql_exception.rs`，类型名不变；crate 全特性编译通过 |

### 3.2 Spring → Vernal（8）——正式批准例外

维护者于 2026-08-04 裁决：**框架替换允许改名前缀（Spring → Vernal）**，以下 8
项全部登记为**正式批准例外**。

| Java 对象 | Rust 等价实现（批准例外） | 承载语义 |
|---|---|---|
| `com.yomahub.liteflow.spi.spring.SpringAware` | `spi/vernal/vernal_aware.rs` / `VernalAware` | `ContextAware` SPI |
| `com.yomahub.liteflow.spi.spring.SpringCmpAroundAspect` | `spi/vernal/vernal_cmp_around_aspect.rs` / `VernalCmpAroundAspect` | `CmpAroundAspect` SPI |
| `com.yomahub.liteflow.spi.spring.SpringContextCmpInit` | `spi/vernal/vernal_context_cmp_init.rs` / `VernalContextCmpInit` | `ContextCmpInit` SPI |
| `com.yomahub.liteflow.spi.spring.SpringDeclComponentParser` | `spi/vernal/vernal_decl_component_parser.rs` / `VernalDeclComponentParser` | `DeclComponentParser` SPI |
| `com.yomahub.liteflow.spi.spring.SpringLiteflowComponentSupport` | `spi/vernal/vernal_liteflow_component_support.rs` / `VernalLiteflowComponentSupport` | `LiteflowComponentSupport` SPI |
| `com.yomahub.liteflow.spi.spring.SpringPathContentParser` | `spi/vernal/vernal_path_content_parser.rs` / `VernalPathContentParser` | `PathContentParser` SPI |
| `com.yomahub.liteflow.spring.ComponentScanner` | `vernal_component_scanner.rs` / `VernalComponentScanner` | 容器组件扫描器 |
| `com.yomahub.liteflow.spring.DeclBeanDefinition` | `vernal_decl_bean_definition.rs` / `VernalDeclBeanDefinition` | 声明式 Bean 定义 DTO |

### 3.3 ReAct（3）——已收口

| Java 对象 | 收口后 Rust | 结论 |
|---|---|---|
| `ReActAgentComponent` | `component/re_act_agent_component.rs` / `ReActAgentComponent` | 2026-08-04 文件名保留 `Re`+`Act` 词边界 |
| `ReActAgentContext` | `component/re_act_agent_context.rs` / `ReActAgentContext` | 同上 |
| `ReActLoggingHook` | `hook/re_act_logging_hook.rs` / `ReActLoggingHook` | 同上；文件头 Java FQN 已修正 |

## 4. JVM 专有脚本对象（15）——逐项裁决完成

2026-08-04 完成 15 个对象的逐项语义裁决。裁决依据：

1. Janino 的"运行期编译 Java 源码"和 liquor/JSR223 的"脚本即 Java 类"
   属于 JVM 专有机制，标记不可迁移。
2. 它们对 LiteFlow 暴露的公共语义——五类节点、参数绑定、ScriptBean、
   编译缓存、校验、卸载/重载和异常——映射到 Rust `ScriptExecutor` 契约。
3. 10 个 body 接口的类型化返回语义由 Rust `ScriptKind` 与五类
   `Script*Component` 完整承接。

| Java 对象 | 模块 | 裁决 | 证据 |
|---|---|---|---|
| `JaninoScriptBody<T>` | script-java | JVM 专有 + Rust 等价：`ScriptKind`/`check_return` | `script/script_executor.rs` |
| `JaninoCommonScriptBody` | script-java | JVM 专有 + Rust 等价：`ScriptKind::Common` | 同上 |
| `JaninoBooleanScriptBody` | script-java | JVM 专有 + Rust 等价：`ScriptKind::Boolean` | 同上 |
| `JaninoSwitchScriptBody` | script-java | JVM 专有 + Rust 等价：`ScriptKind::Switch` | 同上 |
| `JaninoForScriptBody` | script-java | JVM 专有 + Rust 等价：`ScriptKind::For` | 同上 |
| `JavaExecutor` | script-java | JVM 专有（Janino 编译管线） | `rhai_script_executor.rs` |
| `ScriptBody<T>` | script-javax | JVM 专有 + Rust 等价 | `script_kind.rs` |
| `CommonScriptBody` | script-javax | JVM 专有 + Rust 等价：`ScriptKind::Common` | 同上 |
| `BooleanScriptBody` | script-javax | JVM 专有 + Rust 等价：`ScriptKind::Boolean` | 同上 |
| `SwitchScriptBody` | script-javax | JVM 专有 + Rust 等价：`ScriptKind::Switch` | 同上 |
| `ForScriptBody` | script-javax | JVM 专有 + Rust 等价：`ScriptKind::For` | 同上 |
| `JavaxExecutor` | script-javax | JVM 专有 | `ScriptExecutorFactory` |
| `JavaxSettingMapKey` | script-javax | JVM 专有 | `liteflow_config.rs` |
| `JavaxProExecutor` | script-javax-pro | JVM 专有 | `script_executor.rs` |
| `JavaxProSettingMapKey` | script-javax-pro | JVM 专有 | `liteflow_config.rs` |

结论：15 项全部完成逐项裁决并登记等价入口；其中 10 项有 Rust 等价语义承接，
3 个 Executor 与 2 个 SettingMapKey 为 JVM 专有引擎/配置机制。

## 5. 路径与红线扫描

| 检查 | 结果 | 结论 |
|---|---:|---|
| `src/` 以下非 snake_case `.rs` 路径 | 0 | 通过 |
| `compat.rs` | 0 | 通过 |
| `lib.rs`/`mod.rs` 中 `pub struct/enum/trait/type` | 0 | 通过 |
| 生产 `todo!`/`unimplemented!` | 0 | 通过 |
| 文本命中的 wildcard import | 20 | 均为 `#[cfg(test)]` 测试模块的 `use super::*`；生产为 0 |
| Java core 对象唯一 basename | 304/304 | 通过 |
| Java 全工作区直接 basename | 477/500 | 23 项为批准例外或 JVM 专有 |

## 6. 结论与修复顺序

2026-08-04 更新：**名称一致性检查通过**。核心 304 与 EL Builder 18 结构基线
稳定；规则插件 2 项与 ReAct 3 项名称已收口；8 项 Spring→Vernal 已登记为正式
批准例外；15 项 JVM 专有脚本对象已完成逐项裁决。当前 500/500 均有唯一对象裁决，
所有名称差异为 0 或为经批准的显式例外，红线持续为零。

后续维护项（非完成阻塞）：

1. P1：恢复 CodeGraph 方法级检查和 AST 文档/单对象扫描。
2. P1：15 项 JVM 脚本裁决对应的差分用例持续扩充。
