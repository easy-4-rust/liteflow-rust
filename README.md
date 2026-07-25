# liteflow-rust

[LiteFlow](https://github.com/dromara/liteflow)（dromara 轻量组件式规则引擎）的 Rust 语义移植版。
**包结构与对象模型和 Java 版同构**：每种 Condition 是独立对象（一个文件一个类），
实现统一 `Executable` 接口；并行策略是 4 个独立执行器对象；构建期由
`LiteFlowChainELBuilder` 把 EL 语法树组装成 Condition 对象树，与 Java 完全一致。

## 源码结构（与 com.yomahub.liteflow 包一一对应）

```
src/
├── exception.rs                    # exception 包（30+ 异常类型）
├── enums.rs                        # enums 包（ConditionTypeEnum / ParallelStrategyEnum ...）
├── core/                           # core 包
│   ├── node_component.rs           #   NodeComponent trait（isAccess/isContinueOnError/...）
│   ├── flow_executor.rs            #   FlowExecutor（execute2Resp 各重载）
│   └── execute_option.rs           #   ExecuteOption（2.16：requestId/conversationId/eventListener）
├── builder/el/builder.rs           # builder.el 包：LiteFlowChainELBuilder
├── el.rs                           # EL 词法/语法解析（对应 Java 版底层 QLExpress 层）
├── flow/
│   ├── flow_bus.rs                 #   FlowBus + LiteflowMetaOperator
│   ├── liteflow_response.rs        #   LiteflowResponse
│   ├── flow_event.rs               #   FlowEvent（2.15+ 执行事件）
│   ├── flow_event_listener.rs      #   FlowEventListener
│   ├── flow_event_publisher.rs     #   FlowEventPublisher
│   ├── entity/cmp_step.rs          #   CmpStep
│   ├── element/
│   │   ├── executable.rs           #   Executable 接口
│   │   ├── chain.rs                #   Chain
│   │   ├── node.rs                 #   Node（processFlow 语义）
│   │   └── condition/              #   每种 Condition 一个对象：
│   │       ├── then_condition.rs / when_condition.rs
│   │       ├── if_condition.rs / switch_condition.rs
│   │       ├── loop_condition.rs   #   LoopCondition 公共逻辑
│   │       ├── for_condition.rs / while_condition.rs / iterator_condition.rs
│   │       ├── catch_condition.rs / and_or_condition.rs / not_condition.rs
│   │       ├── retry_condition.rs / timeout_condition.rs
│   │       ├── ignore_error_condition.rs
│   │       ├── chain_bind_wrapper_condition.rs  # 子链包装（持有 chain bind 数据）
│   │       ├── bind_wrapper_condition.rs        # Condition 级 bind（2.14+）
│   │       └── pre_condition.rs / finally_condition.rs
│   └── parallel/strategy/          #   flow.parallel.strategy 包
│       ├── all_of.rs               #   AllOfParallelExecutor
│       ├── any_of.rs               #   AnyOfParallelExecutor
│       ├── percentage_of.rs        #   PercentageOfParallelExecutor
│       └── specify_of.rs           #   SpecifyParallelExecutor
├── slot/                           # slot 包
│   ├── slot.rs                     #   Slot（含 conversationId / attachments）
│   ├── databus.rs                  #   DataBus + 请求 ID 生成 + Frame（loop/bind 栈）
│   └── default_context.rs          #   DefaultContext（CmpContext）
├── script/                         # script 包（rhai 脚本节点）
│   ├── script_executor.rs          #   RhaiScriptExecutor（编译/求值/作用域注入/校验）
│   ├── script_component.rs         #   ScriptComponent（5 种脚本类型语义）
│   └── json_convert.rs             #   serde_json ↔ rhai::Dynamic
├── util/el_regex.rs                # util 包：链继承占位符 + EL normalize（2.16）
├── aop/                            # CmpAroundAspect 全局切面
├── lifecycle/                      # 生命周期钩子 SPI（4 种）
├── monitor/                        # MonitorBus 统计报表
├── instance_id/                    # NodeInstanceIdManageSpi
├── rule_plugin/                    # 规则源插件（对应 liteflow-rule-plugin 全家）
│   ├── rule_source.rs              #   RuleSource trait + 轮询热刷新
│   ├── nacos.rs / etcd.rs / zk.rs  #   feature: nacos / etcd / zk（官方 Rust SDK）
│   ├── apollo.rs / redis_source.rs #   feature: apollo / redis
│   └── sql_source.rs               #   feature: sql（rusqlite，表结构对齐）
└── parser/                         # parser 包
    ├── chain_def.rs                #   两阶段构建（链继承 extends）
    ├── local_json_flow_el_parser.rs#   LocalJsonFlowELParser（JSON 规则）
    ├── local_xml_flow_el_parser.rs #   LocalXmlFlowELParser（XML 规则）
    ├── local_yml_flow_el_parser.rs #   LocalYmlFlowELParser（YML 规则）
    └── monitor_file.rs             #   MonitorFile 平滑热刷新
```

## 快速开始

```rust
use liteflow_rust::{FlowBus, cmp};
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
| rhai 脚本节点 | `boolean_script`/`switch_script`/`for_script`/`script` | ✅ |
| XML 规则 | `<chain>/<route>/<body>/<nodes>` | ✅ |
| route 决策表链路 | `add_route_chain` + `execute_route_chain` | ✅ |
| 链继承 | `extends` + `{{占位符}}` | ✅ |
| 子链嵌套 | EL 直接引用子链 id | ✅ |
| 声明式组件 | `register_decl` + EL `cmpId.method` | ✅ |
| 全局 AOP | `register_aspect` | ✅ |
| 生命周期钩子 | 4 种 LifeCycle | ✅ |
| 监控统计 | `bus.monitor().report()` | ✅ |
| 规则源插件 | nacos/etcd/zk/apollo/redis/sql（feature 启用） | ✅ |
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
use liteflow_rust::{FlowBus, rule};
let bus = FlowBus::new();
// 组件注册略
rule::load_json_file(&bus, "rule.el.json").unwrap();
// 热刷新：文件变更后自动重载（先完整解析，再原子替换）
let watcher = rule::RuleWatcher::new(bus.clone(), "rule.el.json").unwrap();
let _h = watcher.watch(std::time::Duration::from_secs(1));
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
`after_process` / `on_error` / `rollback` 默认方法，与 Java NodeComponent 一一对应。

## 运行测试与示例

```bash
export CARGO_TARGET_DIR=/tmp/lf-target  # 本仓库挂载点无执行位，需外挂 target 目录
cargo test                              # 75 个测试（解析/语义/P1/P2/v2.16）
cargo check --features lua              # Lua 脚本引擎（mlua）
cargo check --features nacos            # Nacos 规则源（nacos-sdk）
cargo check --features etcd,zk,sql,redis,apollo  # 其余规则源
cargo run --example order_demo
```

## 许可证

Apache-2.0
