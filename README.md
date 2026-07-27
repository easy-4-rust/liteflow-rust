# liteflow-rust

[LiteFlow](https://github.com/dromara/liteflow)（dromara 轻量组件式规则引擎）的 Rust 语义移植版。
**包结构与对象模型和 Java 版同构**：每种 Condition 是独立对象（一个文件一个类），
实现统一 `Executable` 接口；并行策略是 4 个独立执行器对象；构建期由
`LiteFlowChainELBuilder` 把 EL 语法树组装成 Condition 对象树，与 Java 完全一致。

## Workspace 结构

```
liteflow-rust/
├── liteflow-core/                  # EL、FlowBus、Condition、执行器、SPI 与规则源契约
├── liteflow-derive/                # 组件/声明式组件/Fact/Retry/Fallback 过程宏
├── liteflow-el-builder/            # 18/18 Java EL Builder 对象的 Rust 链式 API
├── liteflow-rule-plugin/           # Nacos/Etcd/ZK/Apollo/Redis/SQL 独立规则源
├── liteflow-script-plugin/         # Lua/JavaScript/Python/QLExpress 独立执行器
├── liteflow-vernal/                # Vernal 生命周期 + Axum/Actix HTTP 适配
├── liteflow-agent/                 # AgentScope ReAct 组件 + 4 个模型提供方子 crate
├── liteflow-benchmark/             # 与 Java POM 一致的 8 个 benchmark 子 crate
├── liteflow-testcase-el/           # 与目标清单一致的 28 个 testcase 子 crate
├── docs/                           # 对象清单、语义矩阵与分阶段路线图
└── Cargo.toml                      # 63 个 workspace crate 的统一清单与依赖
```

## 快速开始

```rust
use liteflow_core::{FlowBus, cmp};
use serde_json::Value;

#[tokio::main]
async fn main() {
    let bus = FlowBus::new();

    // 注册组件（对应 Java 的 @LiteflowComponent("a")）
    bus.register("a", cmp(|ctx| async move {
        ctx.set_data("x", Value::from(1));
        Ok(Value::Null)
    }));
    bus.register("b", cmp(|_| async move { Ok(Value::Null) }));
    bus.register("c", cmp(|_| async move { Ok(Value::Null) }));

    // EL 编排
    bus.add_chain("chain1", "THEN(a, WHEN(b, c))").unwrap();

    // 执行
    let resp = bus.execute("chain1").await;
    assert!(resp.is_success());
    println!("{}", resp.step_str()); // a[0ms]==>b[0ms]==>c[0ms]
}
```

## EL 语法支持

| 语法 | 示例 | 状态 |
|---|---|---|
| 串行 THEN | `THEN(a, b, c)` | ✅ |
| 并行 WHEN | `WHEN(a, b, c)` | ✅ |
| 条件 IF/ELIF/ELSE | `IF(x, a).ELIF(y, b).ELSE(c)` | ✅ |
| 选择 SWITCH | `SWITCH(s).TO(a, "b:tag").DEFAULT(d)` | ✅ |
| 循环 FOR | `FOR(f).DO(x).BREAK(b)` | ✅ |
| 循环 WHILE | `WHILE(w).DO(x).BREAK(b)` | ✅ |
| 迭代 ITERATOR | `ITERATOR(it).DO(x)` | ✅ |
| 并行循环 | `FOR(f).PARALLEL(3).DO(x)` | ✅ |
| 异常捕获 | `CATCH(expr).DO(handler)` | ✅ |
| 前置/后置 | `THEN(PRE(p), a, FINALLY(z))` | ✅ |
| 布尔编排 | `IF(AND(x, OR(y, NOT(z))), a)` | ✅ |
| 重试 | `a.retry(3)` | ✅ |
| 超时 | `WHEN(...).MAX_WAIT_SECONDS(2)` | ✅ |
| ANY 策略 | `WHEN(a, b).ANY(true)` | ✅ |
| PERCENTAGE 策略 | `WHEN(a, b, c).PERCENTAGE(0.5)` | ✅ |
| MUST 策略 | `WHEN(a, b).MUST("a")` | ✅ |
| 忽略错误 | `.ignore_error(true)` | ✅ |
| 节点修饰 | `a.tag("t").data("...").id("a1").bind("k","v")` | ✅ |
| NODE 引用 | `NODE("a")` | ✅ |
| 子流程嵌套 | `THEN(chain_sub_able)` 中以节点形式封装子链 | 见迁移对照表 |
| rhai 脚本节点 | `boolean_script`/`switch_script`/`for_script`/`script` | ✅ |
| XML 规则 | `<chain>/<route>/<body>/<nodes>` | ✅ |
| route 决策表链路 | `add_route_chain` + `execute_route_chain` | ✅ |
| 链继承 | `extends` + `{{占位符}}` | ✅ |
| 子链嵌套 | EL 直接引用子链 id | ✅ |
| 声明式组件 | `register_decl` + EL `cmpId.method` | ✅ |
| 全局 AOP | `register_aspect` | ✅ |
| 生命周期钩子 | 4 种 LifeCycle | ✅ |
| 监控统计 | `bus.monitor().report()` | ✅ |
| 规则源插件 | 独立 `liteflow-rule-plugin`：nacos/etcd/zk/apollo/redis/sql | ✅ |
| 脚本语言插件 | 独立 `liteflow-script-plugin`：lua/js/python/qlexpress + JVM 表达式兼容层 | ✅/🔶 |
| 直接执行 EL | `execute_with_el`（normalize + MD5 匿名链缓存，2.16） | ✅ |
| ExecuteOption | requestId / conversationId / eventListener（2.16） | ✅ |
| 执行事件 | `FlowEvent` + `publish_event` + ExecuteOption.event_listener（2.15+） | ✅ |
| Slot attachment | `set/get/has/remove_attachment`（2.15+） | ✅ |
| Condition 级 bind | `THEN(...).bind(k, v[, override])` / `chain.bind(...)`（2.14+） | ✅ |
| NodeId 校验 | 变量命名规则，`NodeIdUnIllegal`（2.16） | ✅ |
| AND/OR isAccess 过滤 | 不可访问子项排除后再 all/any（2.16） | ✅ |

## 脚本节点（rhai）

```rust
// 对应 Java 的 boolean_script / switch_script / for_script / script 节点
bus.register_script_typed("check", "rhai", ScriptKind::Boolean, "input.score >= 60").unwrap();
bus.register_script("calc", "rhai", "data.total = input.amount * 2").unwrap();
```

脚本作用域注入：`input`（请求参数）、`data`（链路共享数据，变更自动合并回上下文）、
`node_id`、`tag`、`loop_index`、`loop_object`。脚本最后表达式的值为节点返回值，
并按脚本类型校验（boolean_script 必须返回 bool 等）。

非 Rhai 语言先由独立插件显式注册：

```rust
liteflow_script_plugin::register_all()?;
bus.register_script_typed(
    "check",
    "javascript",
    ScriptKind::Boolean,
    "return input.score >= 60;",
)?;
```

Lua 使用 mlua，JavaScript 使用纯 Rust Boa，Python 使用 PyO3 嵌入式
CPython，均经过真实引擎执行测试。QLExpress 直接依赖 crates.io 发布的
`qlexpress 0.1.0-alpha.1`，由真实 lexer/parser/compiler/QVM 执行脚本，并缓存
`SerializableParseCache`；LiteFlow 适配层只负责 DefaultContext、`_meta`、JSON
值和 ScriptBean 桥接。FlowBus 端到端测试已覆盖赋值、循环、复合赋值、条件分支、
五类节点返回和上下文写回，并与 Java QLExpress 4.1.0 做差分验证。Groovy、
Aviator、Kotlin 的 JVM 专属动态能力仍有明确适配边界。

## 决策表链路（route，2.12+）

```rust
bus.add_route_chain("vipChain", "order", "isVip", "THEN(vipDiscount, pay)").unwrap();
let matched = bus.execute_route_chain(Some("order"), json!({"level": 8})).await?;
// 并行求值 order 命名空间下所有 route EL，命中的链路并行执行 body
```

## 规则文件

兼容 LiteFlow 标准 JSON 与 XML 规则格式（含 `nodes` 脚本节点、`route`/`body`/
`namespace`/`enable` 字段），支持平滑热刷新：

```rust
use liteflow_core::{FlowBus, MonitorFile, rule};
let bus = FlowBus::new();
// 组件注册略
rule::load_json_file(&bus, "rule.el.json").unwrap();
// 热刷新：文件变更后自动重载（先完整解析，再原子替换）
let monitor = MonitorFile::new(bus.clone());
monitor.add_monitor_file_path("rule.el.json").unwrap();
monitor.create(std::time::Duration::from_secs(1)).unwrap();
// 关闭时调用 monitor.destroy().unwrap()
```

```json
{
  "flow": {
    "chain": [
      { "name": "chain1", "condition": [ { "type": "then", "value": "a, WHEN(b, c)" } ] }
    ]
  }
}
```

## 组件类型语义（对应 Java 各 Component 子类）

| Java | Rust 返回值约定 |
|---|---|
| NodeComponent（普通） | `Ok(Value::Null)` |
| NodeBooleanComponent（IF/WHILE/BREAK/AND/OR/NOT） | `Ok(Value::Bool(_))` |
| NodeSwitchComponent | `Ok(Value::String("目标id" / "id:tag"))` |
| NodeForComponent | `Ok(Value::Number(_))` |
| NodeIteratorComponent | `Ok(Value::Array(_))` |

组件 trait 还提供 `is_access` / `is_continue_on_error` / `before_process` /
`on_success` / `after_process` / `on_error` / `is_rollback` / `rollback` 默认方法。
Java 通过反射判断是否覆盖 rollback；Rust 端显式返回 `is_rollback = true`，
由执行器按真实执行记录逆序补偿。

## 运行测试与示例

```bash
export PYO3_PYTHON=/path/to/python3.13  # 启用 python feature 时指向可嵌入的 CPython
export DYLD_LIBRARY_PATH=/path/to/python3.13/lib  # macOS Framework/共享库搜索路径
cargo test --workspace --all-features --no-fail-fast # 249 个测试（含 doctest）
cargo test -p liteflow-script-plugin --all-features  # Lua/Boa/CPython 真实运行
cargo test -p liteflow-rule-plugin --all-features    # 含真实临时 SQLite
cargo test -p liteflow-agent -p liteflow-agent-core --all-features # Provider 构造契约 + ReAct 流程
cargo test -p liteflow-benchmark                     # 8 个场景执行真实负载
cargo test -p liteflow-testcase-el                    # 聚合执行全部 28 个 testcase
cargo test -p liteflow-vernal --all-features         # Vernal + Axum + Actix
cargo run -p liteflow-vernal --example vernal_demo   # 启动 HTTP 示例
cargo run --example order_demo
```

`PYO3_PYTHON` 必须指向带可链接共享库或 Framework 的 CPython；仅有解释器
可执行文件但缺少嵌入库时，全特性二进制会在启动阶段失败。

## 许可证

Apache-2.0

### 第三方代码归属

本项目包含来自以下项目的衍生代码：

- **ZeroClaw** (https://github.com/zeroclaw-labs/zeroclaw) —
  `liteflow-agent/liteflow-agent-providers/` 下的 LLM Provider 实现
  （OpenAI/Anthropic/Gemini/GLM/Copilot/Bedrock 等）源自 ZeroClaw，
  遵循其 Apache-2.0 许可（ZeroClaw 为 MIT OR Apache-2.0 双许可，本项目按
  Apache-2.0 接入）。完整归属清单见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
  ZeroClaw 是 ZeroClaw Labs 的商标，本项目与其无官方关联。
- **AgentScope-Rust** (https://github.com/agentscope-ai/agentscope-rust) —
  作为 path 依赖提供 `Model` trait 与 ReAct 运行时，遵循 Apache-2.0。
- **LiteFlow** (https://github.com/dromara/liteflow) — Java 版作为设计参照，
  本项目是其规则引擎语义的 Rust 移植。
