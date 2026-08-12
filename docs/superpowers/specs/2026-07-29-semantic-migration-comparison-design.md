# liteflow-rust 语义迁移对照规范

日期：2026-07-29
状态：活跃
落点：liteflow-rust 全 workspace 功能语义迁移对照

---

# LiteFlow → liteflow-rust 功能语义迁移对照表

> 更新日期：2026-07-29
> 唯一基线：LiteFlow Java v2.16.0 的 500 个生产对象。
> 本表描述行为语义；对象位置见《对象级对照表》，名称缺口见
> 《对象名称一致性检查》。

迁移原则是“Java 行为可追溯，Rust 实现可生产”。Spring 容器、反射、
ThreadLocal、CopyOnWriteHashMap、CompletableFuture 分别映射为 Vernal/显式注册、
trait/过程宏、`Arc` 上下文、并发快照、Tokio 任务；形态改变不自动等于语义完成。

| 状态 | 含义 |
|---|---|
| ✅ | 源码中有真实实现，并存在针对该语义的本地测试 |
| 🔶 | 部分等价、Rust 形态不同或验证范围有限 |
| 🧪 | 仅 fixture/本机真实服务/临时容器验证，不代表生产 |
| ⬜ | 未实现或尚未完成语义裁决 |
| 🚫 | Java/JVM 专有；必须说明 Rust 替代语义，不能静默删除 |

重要结论：本表中的大量核心项已经具备真实实现和测试，但全项目仍不能标记为
“功能语义完全迁移完成”。主要缺口是 JVM 脚本对象裁决、Kotlin 卸载/重载、
规则源集群/鉴权/长稳、SQL 多数据库、Vernal 发布依赖、Agent 真实网络契约和
Java 3,739 个测试文件的逐项迁移账本。

## 一、EL 编排语义（core 核心价值）

| Java 类 | 语义 | Rust 实现 | 状态 |
|---|---|---|---|
| ThenCondition | pre→主体顺序执行→finally；异常记 slot 并抛出；finally 必执行，首个 finally 异常覆盖主体异常并停止剩余 finally | `El::Then` / `ThenCondition::execute` | ✅（主体与 FINALLY 同时失败的异常优先级、停止顺序有真实测试） |
| WhenCondition | 并行执行；分支级 WhenFutureObj 记录成功/失败/超时 | `El::When` + Tokio JoinSet；每分支独立等待并生成真实 `WhenFutureObj`，只记录真正超时项；全局/条件级等待时间、ignoreError 与原始分支顺序已接通 | ✅（混合快慢分支、忽略超时、ANY/PERCENTAGE 首个失败、MUST 缺失回退、ALL access 特例及首错/超时顺序均有真实测试） |
| AllOfParallelExecutor | 全部完成 | 默认策略 | ✅ |
| AnyOfParallelExecutor | 任一 Future 完成即返回（`ANY(true)`，完成结果可以是成功、失败或超时） | 按 `completed` 而非成功数打开门闩 | ✅ |
| PercentageOfParallelExecutor | 指定比例 Future 完成（`PERCENTAGE(p)`，向上取整） | 按 `completed` 计数，随后按原始分支顺序处理失败 | ✅ |
| SpecifyParallelExecutor | 指定分支完成（`MUST("id")`）；指定项全部缺失时回退 ALL | 等待已存在 MUST 分支，空匹配等待全部 | ✅ |
| ParallelStrategyHelper | `OnceLock + RwLock<HashMap>` 缓存四类无状态执行器，`WhenCondition` 统一从 Helper 装配 | ✅ |
| IfCondition | 条件节点结果驱动分支；isAccess=false 直接返回；目标不可 pre/finally | `exec_if` | ✅ |
| SwitchCondition | "id:tag" 目标匹配规则；default；NoSwitchTargetNodeException | `exec_switch` | ✅ |
| ForCondition | 计数循环、DO/BREAK、布尔 PARALLEL 并行提交+启动期 BREAK 检查；默认/false 串行，true 并行 | `exec_for` | ✅ |
| WhileCondition | 条件循环；`parallel: bool` 与 Java LoopCondition 一致 | `exec_while` | ✅ |
| IteratorCondition | 迭代循环与 loopObject；`parallel: bool` 与 Java LoopCondition 一致 | `exec_iter` | ✅ |
| CatchCondition | 捕获执行 DO；无 DO 继续抛出 | `exec_catch` | ✅ |
| FinallyCondition / PreCondition | THEN 内特殊键位；完整 executableList 顺序执行，首错停止 | `El::Pre` / `El::Fin` + 真实 Vec 分组 | ✅（多项替换、空列表、顺序与首错停止有真实测试） |
| RetryCondition | 重试 retryCount 次（总尝试 retryCount+1）；每次 `.retry()` 创建新的 RetryCondition，和 maxWait 保持源码嵌套顺序 | `El::Mods.retry` 独立嵌套层 | ✅ |
| TimeoutCondition | MAX_WAIT_SECONDS/MILLISECONDS | `El::Mods.max_wait_ms` / WhenOpts | ✅ |
| AndOrCondition | AND/OR 布尔短路 | `El::And` / `El::Or` | ✅ |
| NotCondition | NOT 取反 | `El::Not` | ✅ |
| ignoreError | 仅 WhenCondition 并行分支错误策略；其他调用方转型失败 | `WhenOpts.ignore_error` | ✅ |
| NodeInstanceId 管理（同一节点多次出现的实例编号） | `flow::instance_id` 四对象：SPI、基类、文件默认实现、Holder；接入 LiteFlowChainELBuilder、FlowBus、ParserHelper、匿名 EL、route 主体及 Rust 类型化 AST 入口 | ✅（MD5 + serde JSON 文件持久化、跨 FlowBus 恢复、自定义 SPI 替换、规则源/匿名/route/AST 构建和默认关闭零调用均已真实测试；route 判断节点不编号；AST 使用带版本前缀的稳定 serde 摘要且始终立即编译） |
| ChainBindWrapperCondition | 首个属性为 bind 的子链包装；后续 tag/id 写入同一 Condition，显式 ID 进入真实执行对象 | ✅ |

## 二、EL 解析

| Java | Rust | 状态 |
|---|---|---|
| QLExpress4 作为 EL 解析引擎（2.11+） | crates.io `qlexpress 0.1.0-alpha.1` 的真实 lexer/parser/compiler/QVM；`QlExpressUtils` 注册 17 个主函数并通过 NativeObject 将链式扩展分派给独立强类型 Operator | ✅ |
| LiteFlowChainELBuilder.setEL() | `parse_el()` / `FlowBus::add_chain()` | ✅ |
| validate()/validateWithEx() 校验 | `FlowBus::validate_el()` + `LiteFlowChainELBuilder::validate/validate_with_ex`；后者绑定当前 FlowBus 校验节点、声明式组件和子链注册状态，并真实构建临时可执行树执行 Operator 节点类型约束 | ✅ |
| EL Builder API（代码式组装） | 独立 `liteflow-el-builder` crate：ELBus + 18 个一文件一对象包装器；支持 Java 完整语句及可直接解析的运行时表达式 | ✅ |
| builder/el/operator（34 + base 2） | 36 个独立 Rust 对象；`el/parser.rs` 只负责 QVM 链式扩展分派，旧手写 lexer/token/递归下降 parser 已移除 | ✅ |
| LiteFlowNodeBuilder + builder/prop | `LiteFlowNodeBuilder` + serde `NodePropBean`/`ChainPropBean`；JSON/YAML/XML 节点构建统一接入 | ✅（Java class 反射改为显式 NodeComponent） |
| 非法 EL 的精细化错误提示（ElRegexUtil 等） | QLExpress 原生词法/语法诊断连同 EL 原文返回；未注册节点继续在强类型构建阶段报错，多行与未闭合表达式有真实回归测试 | ✅ |

## 三、组件模型

| Java | Rust | 状态 |
|---|---|---|
| NodeComponent.process() | `NodeComponent::process(&self, ctx) -> Result<Value>` | ✅ |
| NodeBooleanComponent / NodeSwitchComponent / NodeForComponent / NodeIteratorComponent / NodeBreakComponent | 单一 trait，返回值类型区分语义 | ✅ |
| isAccess() | `is_access()` | ✅ |
| isContinueOnError() | `is_continue_on_error()` | ✅ |
| beforeProcess / afterProcess / onError / rollback | 同名默认方法 | ✅ |
| @LiteflowComponent 声明式注册 | `#[liteflow_component]` 生成 FlowBus 显式注册入口 | ✅ |
| @LiteflowMethod / @LiteflowCmpDefine（DeclComponentProxy） | `#[liteflow_cmp_define]` 生成静态分派及 Method/Parameter/DeclWarp 元数据；方法级 `value/node_id/node_name/node_type` 会按 nodeId 生成多个共享原始对象的代理，主方法决定组名称/类型，Java 风格 EL 直接使用 `nodeId`。访问/结束/错误继续/前后置/成功/错误/回滚均进入真实 Node 生命周期，`ON_ERROR` 接收真实 `LiteflowError`，方法级 retry 进入 NodeExecutor；`GET_DISPLAY_NAME` 改写真实步骤名称，`GET_NODE_EXECUTOR_CLASS` 按类名选择注册的自定义执行器。旧 `cmpId.methodName` 仅保留兼容。剩余工作是扩大非法声明夹具并把 Java declare testcase 自动差分接入验收 | 🔶 |
| @LiteflowFact 参数注入 | `#[liteflow_fact("beanName")]` 从 Slot bean 注入 `Arc<T>` | ✅ |
| @FallbackCmp 降级组件 | `#[fallback_cmp]` + `FallbackNode`；按 COMMON/BOOLEAN/SWITCH/FOR/ITERATOR 位置选择 | ✅ |
| @LiteflowRetry 注解重试 | `#[liteflow_retry(n, for = [...])]`；另保留 `.retry(n)` EL 修饰 | ✅ |
| cmpData / tag / bindData | `NodeRef.data/tag/bind` + `ctx.cmp_data()/tag()/bind_data()` | ✅ |
| 脚本组件族（ScriptCommon/Boolean/Switch/ForComponent） | Rhai、Lua、JavaScript、Python 已有独立运行时；QLExpress 直接使用 crates.io `qlexpress 0.1.0-alpha.1` 的 lexer/parser/compiler/QVM 和可序列化编译缓存，LiteFlow 层仅桥接 DefaultContext、完整 Java `bindParam`（含 serde context bean、主/子链请求、slot/chain/node/cmp/loop 元数据）与受控 ScriptBean，真实 FlowBus 已覆盖循环/复合赋值/分支/五类返回并保留 Java 4.1.0 差分，独立 QlExpress + Vernal testcase 与 benchmark 均执行真实 QVM；Aviator 已覆盖 Java 基线的 use/DateUtil/println/setData；Groovy 已覆盖 def/基础类型、DefaultContext、`_meta.cmpData`、println、if/else、FOR/ITERATOR 循环元数据，并通过现有 ScriptBeanProxy/Manager 实测全局及执行级直接对象方法调用、执行级优先和 include/exclude 拒绝语义；Kotlin 已对照 Java `common/cmpdata/scriptbean/scriptmethod/contextbean/validate/throwException/refresh` testcase 覆盖 val/var、显式基础类型、表达式/块函数、块级 if/else、bindings DefaultContext、`_meta`/cmpData、ScriptBean、普通/Boolean/Switch/For 与 WHILE BREAK；`@ScriptMethod` 等价定义由 Vernal `ScriptMethodBeanProcess` 按注解 value 分组注册，Kotlin 已通过 `demo`/`demo2` 两组别名执行真实方法及依赖调用；`Arc<RwLock<serde_json::Value>>` 映射可变 Java 上下文 Bean，JavaBean getter/setter 读写 Slot 和响应持有的同一对象，且请求级 Bean 按 Java putIfAbsent 语义优先于同名全局 ScriptBean；`throw TestException("T01", "测试错误")` 会还原为带业务码的 LiteFlowException；XML 元数据刷新会用新脚本和新增节点原子替换同 ID Chain，FlowBus 与独立 Kotlin + Vernal testcase 已真实执行上述链路；任意 JVM 类实例化、非 serde 对象方法、完整 Kotlin 标准库和动态 classpath 仍为明确边界 | ✅(rhai/lua/js/python/qlexpress) / 🔶(groovy/kotlin/aviator) |

## 四、上下文与数据总线

| Java | Rust | 状态 |
|---|---|---|
| DataBus / Slot 池 | `DataBus` 负责并发分配、1.75 倍扩容、查询和回收；Class 工厂/Bean 实例入口均创建真实 Slot；`Ctx` 以 `Arc<Slot>` 跨 await 传递；Drop 租约保证异步取消安全 | ✅ |
| contextBean（多上下文 bean） | `beans: DashMap<String, Arc<dyn Any>>` + `ctx.bean::<T>(name)` | ✅ |
| requestData | `ctx.request_data::<T>()` | ✅ |
| slot.exception | `Ctx::set_exception` / `resp.slot_exception()` | ✅ |
| setIsEnd / ChainEndException | `ctx.end_chain()` + `LiteflowError::ChainEnd` | ✅ |
| loopIndex / loopObject 栈（含 depth 查询） | `Frame` 原生执行栈 + Java `loopIndexTL`/`loopObjectTL` 两套独立兼容栈；同一 Condition 更新不重复压栈，分别 remove 不会误删另一栈或父层 | ✅（嵌套两层的独立移除与 depth 查询有对象级测试） |
| TransmittableThreadLocal 跨线程传递 | Arc 共享 + Frame clone（async 天然安全） | 🔶 |

## 五、执行入口与响应

| Java | Rust | 状态 |
|---|---|---|
| FlowExecutor.execute2Resp(chainId) | `FlowBus::execute` | ✅ |
| FlowExecutor.init / reloadRule | `FlowExecutor::init` 执行组件 SPI、ID、Parser、缓存、启动钩子与可选 MonitorFile；`reload_rule` 以非启动模式重读规则 | ✅ |
| FlowExecutor.invoke(nodeId, slotIndex) | `FlowExecutor::invoke` 从 DataBus 取得共享 Slot，并经 Node/NodeExecutor 完整生命周期执行 | ✅ |
| FlowExecutor.getStartUpPhase | 共享 `Arc<AtomicBool>` + Drop 守卫，成功和错误返回均复位 | ✅ |
| execute2Resp(chainId, requestData, contextBeans...) | `execute_with_data` / `execute_with` | ✅ |
| 带超时执行 | `execute_timeout` | ✅ |
| LiteflowResponse（success/message/cause/steps） | `LiteflowResponse` | ✅ |
| getExecuteStepStr(WithTime) | `step_str()` | ✅ |
| FlowExecutorHolder 单例 | `OnceLock<RwLock<Option<FlowExecutor>>>` 全局持有器（Java 单例语义）+ 显式 `FlowBus` 多实例入口 | ✅ |
| 路由链路（route EL，2.12+） | `Chain.execute_route` + `FlowExecutor::execute_route_chain`（并行求 route、命中并行执行 body、RouteChainNotFound/NoMatchedRouteChain） | ✅ |

## 六、规则与热部署

| Java | Rust | 状态 |
|---|---|---|
| Class/Local JSON、XML、YML FlowELParser、两个工厂与容器 PathContentParser | 六个 parser 对象固有 `parse_main` 复用真实内容源/PathContentParser；Local 支持绝对文件、Ant 通配符、裸相对资源与显式 `classpath:`；Vernal 对应 SpringPathContentParser，支持 `classpath*:` 跨资源根多文件、同批扩展名校验和优先级 1 Holder 注册。两类实现均已从真实资源构建 Chain | ✅ |
| MonitorBus + MonitorFile 平滑热刷新 | `MonitorFile`（FlowBus 隔离弱单例；`on_file_create/change/delete` 与多路径 mtime 轮询共享状态，完整解析后替换、失败保留、删除清理与 destroy） | ✅ |
| XML / YML 规则解析 | `rule::load_xml_str/file`（quick-xml）/ `rule::load_yml_str/file`（serde_yaml），对齐 ParserHelper 语义 | ✅ |
| Nacos/ZK/Apollo/Redis/Etcd/SQL 规则插件 | 独立 `liteflow-rule-plugin`；Nacos/ZK/Etcd/Redis 原生监听，Apollo 指纹 watcher；SQL/Nacos/ZK/Etcd/Redis 已做本机真实服务闭环，Apollo 已做真实 HTTP 协议夹具 | ✅/🔶 |
| Chain 缓存生命周期 | Vernal `chainCacheEnabled/chainCacheCapacity` 接入 `ChainCacheLifeCycle`；LRU 淘汰已物化 Chain，`RuleDefinitionPlan` 在下次执行时按依赖闭包重新构建；最终物化使用 `build_immediately`，不会因其他上下文切换全局 `parse_mode` 而重新生成空占位链 | ✅ |

## 七、监控与生命周期

| Java | Rust | 状态 |
|---|---|---|
| CmpStep（步骤轨迹：耗时/成功/异常/线程名） | `CmpStep` | ✅ |
| MonitorBus 统计报表 | `monitor::MonitorBus`（record/report + CompStatistics）；Vernal 按 `enableLog/delay/period/queueLimit` 自动调度并随 Context 关闭 | ✅ |
| ICmpAroundAspect 切面 | `aop::ICmpAroundAspect` 独立对象（before_process/on_success/on_error/after_process），`FlowBus::register_aspect` | ✅ |
| 生命周期 SPI（PostProcess*LifeCycle） | 5 类钩子（node_build/chain_build/flow_execute/chain_execute/script_engine_init），FlowBus/脚本构建注册；node_build 与 chain_build 的 before 修改进入最终对象；flow_execute/chain_execute 同时接收 chainId 与真实 Slot，嵌套子链独立触发 chain_execute | ✅（构建顺序、同一对象可见性、Slot 参数、主链/子链嵌套顺序及缺失 Chain after 均有真实测试） |

## 八、v2.16.0 关键增量语义

| Java 类/语义 | Rust 实现 | 状态 |
|---|---|---|
| FlowExecutor.execute2RespWithEL（直接执行 EL，normalize + MD5 匿名链缓存） | `execute_with_el[_data/_full]` + `FlowBus::get_chain_id_by_el_md5` + `ElRegexUtil::normalize` + Chain.el/el_md5；匿名 Chain 通过标准 LiteFlowChainELBuilder 构建 | ✅（缓存复用及启用 NodeInstanceId 时真实 SPI 读写/步骤编号均有测试） |
| ExecuteOption（requestId/conversationId/autoConversationId/contextClass/contextBeans/eventListener） | `core::ExecuteOption`（builder 风格，contextClass 以 Default 延迟工厂在执行期实例化，resolve_conversation_id 语义一致） | ✅ |
| execute2Resp(chainId, param, ExecuteOption) | `FlowExecutor::execute_with_option` / `FlowBus::execute_with_option` | ✅ |
| execute2RespWithRid（组件内同一 requestId 调子链） | `execute_with_rid` | ✅ |
| FlowEvent / FlowEventListener / FlowEventPublisher（listener 存 Slot attachment，publish 回调） | `flow::FlowEvent`（builder）/ `FlowEventListener`（含闭包适配）/ `FlowEventPublisher` + `Ctx/CmpContext::publish_event` | ✅ |
| DefaultContext（ConcurrentHashMap、null 拒绝、getDataMap） | 独立 `slot::DefaultContext`；`DashMap<String, serde_json::Value>` 并发存储、`NullParamException` 与安全拥有型快照；Rust 节点执行视图拆分为 `cmp_context.rs` | ✅ |
| Slot.setAttachment/getAttachment/hasAttachment/removeAttachment | `Slot::set_attachment/get_attachment/has_attachment/remove_attachment`（DashMap 实现） | ✅ |
| conversationId（Slot 字段 + ConversationIdGenerator） | `Slot.conversation_id` + `gen_conversation_id`（唯一短随机串，NanoId 语义） | ✅ |
| Condition 级 bind（`THEN(...).bind(k,v[,override])`，bindData 存 Condition，查找沿 condition 栈顶向下） | `Mods.bind` + 按 key 保存的 `bind_override_keys` + `BindWrapperCondition` + `Frame.bind` 栈 | ✅ |
| Chain 级 bind（bindItem 为 Chain 时包装 ChainBindWrapperCondition 持有 bindData，避免子链共享污染） | `ChainBindWrapperCondition::put_bind_data`，builder 自动转移 NodeRef.bind | ✅ |
| bind override=true 清除子 Node 同 key bind | 构建期 AST 递归清除（`clear_node_bind`，对齐 BindOperator.clearNodeBindData） | ✅ |
| NodeComponent.getBindData 查找顺序（Node → condition 栈） | `CmpContext::bind_data`：非空白 node.bind → `frame.find_bind`（栈顶向下）；组件数据与 bind 均按 Java `StrUtil.isBlank` 处理 | ✅ |
| NodeIdUnIllegalException（nodeId 变量命名规则校验） | `LiteflowError::NodeIdUnIllegal` + `FlowBus::try_register`（`register` 非法 id 直接 panic，对齐 Java 注册期抛错） | ✅ |
| AND/OR 求值前按 isAccess 过滤（isAccess 异常=排除；Node.setAccessResult） | `Executable::is_access` + `set_access_result`；AND/OR 与非 ALL WHEN 预过滤后把结果缓存到 Frame，Node 在 NodeExecutor 重试前只求值一次并于执行结束清理；访问异常继续执行 `isEnd/continueOnError` 优先级 | ✅（有副作用 isAccess、重试、异常与缓存清理均有测试） |
| ScriptValidator.validate/validateWithEx | 独立 `script::validator::ScriptValidator`；保留隐式单语言、多语言拒绝、`ScriptTypeEnum` 显式重载、异常响应及 Map boolean 短路批量语义，另提供 language 诊断 Map 扩展 | ✅ |
| SwitchCondition.getTargetList / NodeSwitchComponent.getTargetList | `SwitchCondition::get_target_list` + `NodeSwitchComponent::get_target_list`；目标 ID 随不可变 `Frame` 进入 `CmpContext::switch_target_list`，已通过真实 SWITCH 链路验证 | ✅ |
| ChainCacheLifeCycle（Caffeine 缓存清理过期链） | `lifecycle::impl::ChainCacheLifeCycle`：LRU 活跃状态、淘汰监听与幂等清理回调 | ✅ |
| ThreadPoolOperator 自定义线程池 | WHEN 记录于 `WhenOpts.thread_pool`、循环记录于 `Mods.thread_pool`，构建后由 `ExecutorHelper` 选择并缓存真实有界 Tokio 执行器；Condition > Chain > 全局优先级及单 worker 并发峰值已有实测 | ✅ |
| ParserHelper 两阶段重构（parse → compile 分离，isAbstract/isCompiled） | 两阶段计划及伴随 ChainDef 已归入 `parser_helper.rs` 同文件，语义等价 | ✅ |
| liteflow-react-agent / property.agent.* | 独立 `liteflow-agent`，AgentScope ReAct 组件 + 会话隔离；Java 4 个 provider 模块逐对象落位，Rust 另有 provider-core、GLM、Copilot、Bedrock、OpenRouter、Telnyx、Compatible 等扩展 | ✅/🔶 |

## 九、框架与 Agent 集成

- liteflow-spring / Spring Boot 3 与 Boot 4 starter 已迁入
  `liteflow-vernal`，以 Vernal 生命周期和 Axum 映射容器语义；Actix 是 Rust
  补充适配。Java v2.16.0 没有 Quarkus 生产模块，不建立虚假对象对照
- Spring 容器 SPI：`VernalAware`、`VernalPathContentParser`、
  `VernalCmpAroundAspect`、`VernalContextCmpInit`、
  `VernalDeclComponentParser`、`VernalLiteflowComponentSupport` 已覆盖
  `spi.spring` 全部 6 个对象。全局切面保持 Java 同步回调和
  before/success/error/after 顺序，并绑定到每个 Vernal `FlowBus`，防止多个
  并行应用上下文通过进程级 Holder 相互覆盖；托管节点以同一
  `Arc<dyn NodeComponent>` 在规则解析前进入 `add_managed_node`。声明式包装
  对象同时进入 Vernal 命名容器和真实 FlowBus 代理执行链；普通组件名称恢复
  `ComponentInitializer → LiteflowComponentSupportHolder` 的运行时调用顺序
- Spring 顶层容器对象：`VernalDeclBeanDefinition` 在组件扫描前经声明解析
  Holder 完成过滤、校验、按 nodeId 拆分注册；`VernalComponentScanner`
  通过 `LiteflowScannerProcessStepFactory` 按 1→7 优先级执行首个匹配步骤，
  完成声明代理、普通节点、全局切面、ScriptBean、ScriptMethod 和 LifeCycle
  的真实注册，再把托管节点交接给 ContextCmpInit；`LiteflowSpiInit`
  在六个 Holder 注册后执行统一预加载。三者均为独立文件和真实 Vernal 单例，
  `printBanner` 已接入配置。`process/*` 12 个 Java 对象均已逐文件落位；
  `LiteflowComponent` 已由 `liteflow-derive` 的独立 annotation 对象提供编译期
  等价注册，不模拟 JVM classpath 反射
- Spring Boot starter：5/5 Java 对象已按原包结构独立落位。
  `LiteflowProperty`/`LiteflowMonitorProperty` 保留默认值、camelCase
  反序列化与属性访问语义；`LiteflowPropertyAutoConfiguration` 逐字段生成
  Vernal 统一配置；`LiteflowMainAutoConfiguration` 保留
  `liteflow.enable=true` 条件；`LiteflowExecutorInit` 作为真实 Vernal
  生命周期执行幂等规则初始化。`checkNodeExists` 与 Java starter 一样保留在
  属性层，但 Java 2.16.0 自动配置本身未把它写入核心 `LiteflowConfig`，Rust
  因而也不伪造核心接线
- Spring Boot 4 starter：另有 5/5 Java 对象按 `springboot4/` 独立落位。
  它们不是 Boot 3 type alias：主属性、监控属性、字段合并和生命周期均有本包
  真实实现；主自动配置在组合门面选择 Boot 4 初始化类型，容器测试证明不会
  同时注册 Boot 3 初始化 Bean
- Solon 插件：Java v2.16.0 的 15/15 个生产对象均已有独立 Rust 实现证据。
  其中同 FQN
  `LiteflowComponent` 复用 `liteflow-derive` 编译期 annotation；
  `SolonNodeIdHolder` 以 ContextAware 附件保持应用上下文隔离；
  `LiteflowMonitorProperty`、`PathsUtils`、`ResourceUtils` 分别覆盖 serde
  属性绑定、真实通配资源扫描和协议常量；`LiteflowProperty` 保留完整字段、
  缺省回退及通配 setter 语义，两个自动配置完成逐字段合并、MonitorBus 创建
  与 `parseOnStart=false` 首次执行解析。`XPluginImpl` 完成默认属性装载、
  禁用短路及各类注册编排，6 个 Solon SPI 通过独占分支进入容器并覆盖组件
  执行、切面和真实资源读取。这里证明的是 Rust/Vernal 宿主映射，不等同于
  JVM Solon 插件加载或真实 Solon 服务器 E2E
- JSR223 / ScriptBean：Rhai 留在 core；Lua/Boa JavaScript/PyO3 Python 位于独立脚本插件；JVM 语言只承诺公共表达式子集
- liteflow-react-agent：已迁入 `liteflow-agent`，以 AgentScope `Model`/ReAct 对齐
  Agent 编排；配置工作区后会按 conversation 自动创建隔离目录并注册
  `read_file/write_file/list_files/delete_file` 以及受控 `execute_shell_command`，
  真实文件系统、子进程超时强杀、输出上限与越界防护已测试；Shell 策略不是
  操作系统级沙箱，生产使用仍需进程或容器隔离；Redis/MySQL 记忆工厂支持按
  Java beanName/DataSource 名称解析宿主注册的真实 AgentScope Session，缺失
  配置或资源会明确失败，不降级为内存模式。provider 构造已验证，真实外部模型
  网络调用未验证；Redis/MySQL 已完成本机密码鉴权单节点的单值、列表、存在性、
  删除及错误凭证拒绝闭环，TLS、Redis ACL 多用户、集群与生产长稳未验证

## 十、脚本引擎语义与现成组件

| Java 能力 | Rust 实现 | 当前证据 | 未闭环语义 |
|---|---|---|---|
| QLExpress4 Chain EL | `qlexpress = 0.1.0-alpha.1` + 强类型 Builder | lexer/parser/compiler/QVM、核心 EL 与脚本执行已有真实代码和测试 | 继续做 Java v2.16.0 全 operator 差分与版本锁定 |
| QLExpressScriptExecutor | `liteflow-script-qlexpress` | 普通/Boolean/Switch/For、bindParam、上下文与缓存测试 | 并发缓存、卸载/重载和错误文本差分 |
| Lua | `mlua 0.12` | 真实 Lua 执行 | 超时/内存限制、模块白名单、统一 remove/reload |
| Python | `pyo3 0.29` | 真实 CPython 执行 | GIL/多线程、解释器初始化、模块白名单、取消和资源上限 |
| JavaScript/GraalJS | `boa_engine 0.21` | 真实 JavaScript 执行 | GraalJS 专属 host interop 不可等同；补资源限制和生命周期 |
| Groovy | 当前为受控 Rhai 适配；候选 `groovy 0.1.0`/`groovyrs` | 基础语法、上下文、循环元数据、ScriptBean 测试；候选 crate 提供 parse/compile/run_str 与 fusevm JIT | 候选 `run_str` 私有创建 VM，尚无 bindings、host callback、写回、取消和 remove/reload；完成适配 POC 前不替换 |
| Kotlin | 受控 Rust 解释层 | val/var、类型转换、上下文、五类节点等测试 | `remove` 后缓存/源码/nodeIds/重载路由需补齐 |
| Aviator | 受控 Rust 解释层 | 部分函数、日期、上下文测试 | 完整 Aviator 语法/函数生态未证明 |
| Janino/JSR223 | 🚫 JVM 机制；映射到 Rust `ScriptExecutor` 契约 | 15 个 Java 对象已识别 | 尚未逐对象裁决公共语义，不能计为完成 |

QLExpress、mlua、PyO3、Boa 已经提供真实执行引擎，不应从头实现。`groovy 0.1.0`
是值得优先验证的真实无 JVM Groovy 前端，但 LiteFlow 需要的宿主绑定和生命周期
API 尚不能从其公开 `run_str` 直接获得。ANTLR4 Runtime 仅在必须复用 Java grammar
时引入；当前 Chain EL 已由 QLExpress4 覆盖，不增加第二套 parser。
`javascript 0.1.13` 作为 Boa 的差分候选，不直接替换；其真实仓库是
`https://github.com/ssrlive/javascript`，不是 `mlua-rs/mlua`。

## 十一、校验、AOP、HTTP 与序列化组件

| 组件 | 结论 | LiteFlow 边界 |
|---|---|---|
| `validator`/derive | P1 采用 | 配置对象和规则源 VO 使用声明式字段/结构校验；Java 异常类型、错误码、文本和校验顺序敏感逻辑保留显式校验 |
| `aspect-core 0.1.2` | 不替换 `ICmpAroundAspect` | 通用 Aspect/JoinPoint 使用 `Any` 结果和通用 Advice；LiteFlow 需要 CmpContext、before/success/error/after 及严格调用顺序，只允许在 Vernal 扩展层做可选桥接 |
| `http 1.4` | 按需直接依赖 | 仅用于跨 Axum/Actix/Tungstenite 的公共请求、响应、状态码和 Header 类型；内部实现已有框架重导出时不增加重复依赖 |
| `serde`/`serde_json` | 保持 | 已承担 Jackson 映射；继续补 `rename`、`skip`、默认值、自定义 Serializer 和错误文本差分 |
| `quick-xml 0.41` | 保持 | 已承担 XML；ParserHelper 聚合、扩展名校验、完整解析后原子发布仍属于 LiteFlow 领域语义 |
| `antlr-rust-runtime` | 暂不引入 | 仅当必须消费 Java ANTLR grammar 且 QLExpress/quick-xml/serde 无法覆盖时使用 |

## 十二、规则源、任务与数据访问组件

| 语义 | 当前实现 | 迁移决策 |
|---|---|---|
| Apollo HTTP | 当前协议实现和本机 fixture | 统一到 workspace 已有 `reqwest` 异步客户端，补真实集群/鉴权/灰度 |
| watcher/polling/Monitor | 多种 Tokio 任务实现 | 使用 `tokio-util::CancellationToken`；Vernal 宿主使用 `AsyncTask`/`ScheduledTask` 托管 |
| SQL | `rusqlite`，真实 SQLite | 抽象到 `sqlx` 或 `rbdc`，覆盖 MySQL/PostgreSQL/SQLite；不自造连接池 |
| SPI 发现 | 显式 factory/registry 为主 | ScriptExecutor/ModelProvider 可使用 `inventory`，冲突和禁用必须可诊断 |
| 配置绑定 | serde + 手工合并 | 集成层复用 Vernal `ConfigurationProperties`，core 保持无容器依赖 |
| 缓存 | DashMap/RwLock/自有 LRU | Moka 只用于可再生有界缓存；权威 FlowBus/注册表不直接替换 |
| 观测 | `tracing` 为主 | 增加 `metrics`；业务可选 DDD4R 观测，core 不强耦合 |
| 宿主表达式 | Vernal SpEL | 只用于配置/条件，不替换 LiteFlow Chain EL |

## 十三、当前缺口清单

| 能力域 | 缺口 | 优先级 |
|---|---|---|
| 对象命名 | 13 个非 JVM 名称差异；15 个 JVM 对象未裁决 | P0 |
| 方法语义 | 两侧无 `.codegraph/`，当前无法复验历史方法审计 | P0 |
| 脚本 | Kotlin remove/reload；各引擎统一取消、资源限制和缓存生命周期 | P0 |
| Groovy | 完成 `groovy 0.1.0` bindings/ScriptBean/写回/五类节点/remove/reload POC 后再决定替换 | P0 |
| 规则热更新 | 统一取消与关闭；Apollo 真实服务；集群故障和长稳 | P0/P1 |
| SQL | 只有 SQLite 实证，MySQL/PostgreSQL 未闭环 | P1 |
| Vernal | 配置绑定统一、优雅停机、多上下文、发布 path dependency | P1 |
| Agent | provider 构造多于 Java 基线，但真实网络、凭证刷新、限流/重试未全证 | P1 |
| 测试 | Java 3,739 个测试文件尚无逐项迁移账本 | P1 |
| 生产 | TLS/ACL、故障注入、性能、长稳、安全与发布认证 | P2 |

## 十四、测试证据口径

- “测试文件存在”不等于“本轮已执行”；文档只记录命令、日期和结果。
- 本地 fixture 标记 🧪；临时单节点容器标记 🧪；真实集群故障/长稳完成后才能升级。
- Java/Rust 差分测试优先于单边 Rust 单测数量。
- `scripts/audit_java_method_parity.mjs --self-test` 于 2026-07-29 通过；该命令只
  验证审计脚本自身，不验证方法迁移。
- 旧文档记录过 core 335、Vernal 36、workspace 630 项测试等历史结果。2026-07-29
  文档刷新时尚未完成全 workspace 重新执行，因此这些数字不再作为当前通过结论。
- 2026-07-29 `cargo check --workspace --all-features` 已通过。为恢复全特性构建，
  `reqwest 0.13.4` 改用 `rustls` 并启用 OAuth 所需 `form`；`rusqlite` 锁定
  `=0.32.1`，与 AgentScope `sqlx 0.8.x` 共用兼容的 `libsqlite3-sys 0.30.x`。
- 同轮完成 syn 3 receiver、PyO3 0.29 `Python::attach`、etcd-client 0.19
  `WatchStream`、redis 1.4 builder/accessor、chacha20poly1305 0.11 `Generate`、
  hmac 0.13 `KeyInit` 和 SHA-1 输出格式的 API 迁移。
- 目标测试结果：Agent Provider 193/193、Bedrock 9/9、Etcd 3/3、Redis 14/14、
  SQL 7/7、Groovy 3/3 通过；Python 在补充本机
  `DYLD_LIBRARY_PATH=/Library/Frameworks/Python.framework/Versions/3.13/lib`
  后通过。`liteflow-derive` 运行测试 6/7，通过项之外的
  `orderCmp.isVip` 用例将同一声明式组件混用 Common/Boolean 方法，而 Java
  `DeclComponentProxy` 要求同一 nodeId 的 nodeType 唯一，需单独按 Java 语义修订。
- 2026-07-29 已按 Java 不变量把该用例拆为不同 nodeId，并让
  `#[liteflow_cmp_define(..., node_type = "boolean")]` 生成
  `ProcessBoolean + NodeTypeEnum::Boolean` 元数据；derive 定向测试 7/7 通过。
  这只证明类级节点类型切片，不证明方法级声明与全部生命周期完成。
- 同日完成修复后重新执行全 workspace/all-features 的 `cargo llvm-cov`，
  全部测试通过；当前 region 78.36%、function 77.29%、line 78.75%。
  当前仍不得标记为 100% 覆盖或全量行为验收。
- 基线发现的 Vernal LRU/组件输入默认并行失败包含三处根因：运行时执行器重复读取
  全局配置、`RuleDefinitionPlan` 最终物化阶段二次读取全局 `parse_mode` 并生成
  空占位链、业务切面存入进程级 Holder。现分别通过隔离执行器、
  `LiteFlowChainELBuilder::build_immediately` 和 `FlowBus` 级切面完成修复。
  新增并发回归每次执行 128 轮交错；完整 Vernal 默认并行测试二进制连续 20 次
  36/36，core 物化回归 12/12，全特性覆盖构建 37/37。该路径可标记完成，但
  `ContextAware`、`PathContentParser`、`ContextCmpInit` 等剩余 Holder 仍需逐项
  做作用域裁决，不能泛化为所有宿主全局状态已经闭环。
- 同日 core + Vernal 全特性覆盖率增量审计为 region 76.46%、function 79.52%、
  line 78.40%；这是包级定位数据，不是 workspace 的最终验收结果。
- 声明式组件切片完成后，core + derive 全特性覆盖率增量审计为 region 73.10%、
  function 78.66%、line 75.18%，仍缺 5,095 行；所有运行测试与 7 个 trybuild
  编译契约通过，但该结果明确不能作为 100% 验收。
- `LiteFlowChainELBuilder` 源码复核新增三项闭环：包装型 `retry/maxWait`
  route 按 Java 最终对象类型拒绝，`ignoreError` 则因调用方不是 WhenCondition
  在 Operator 阶段拒绝；无 route EL 重编译会清除旧 routeItem；主体构建失败会
  清理实例编号临时状态。带 tag 的 Boolean Node route 仍合法，相关路径均有
  真实回归。
- 同轮 `cargo test -p liteflow-core --all-features` 全部通过；新生成的 core
  覆盖率为 region 68.56%、function 74.87%、line 71.41%，仍缺 5,945 行。
  这是 core 包定位基线，不是 workspace 100% 验收。
- 后续 Operator 审计确认 `IGNORE_ERROR` 的 Java 调用方只允许
  `WhenCondition`，此前 Rust 普通 Condition 包装语义属于误迁移。现已删除无
  Java 对偶的 `IgnoreErrorCondition` 及 `Mods.ignore_error`，普通 Condition
  调用返回 Java 对等转型错误。
- `OperatorHelper::add_mods` 不再合并 `retry/maxWait` 包装字段。解析结构测试
  覆盖两种反向顺序和重复 retry；真实运行测试证明
  `maxWait → retry` 首次超时后会重试并成功，而 `retry → maxWait` 由外层超时
  取消整个内部执行且只调用组件一次。
- 本次变更后重新执行 core 全特性覆盖率：region 67.00%（19,816/29,578）、
  function 73.58%（2,902/3,944）、line 69.86%（14,871/21,288），仍缺
  6,417 行。LLVM 对泛型/异步实例化的分母随新增调用路径增长，因此不能把百分比
  下降解释为测试回退；100% 目标仍未达到。
- 属性型 Mods 现在只在类型化操作符分派时临时穿透，操作完成后恢复到同一
  Condition；真实 retry/maxWait 包装不穿透。由此补齐
  `WHEN.id.ignoreError/any/percentage/must/maxWait`、
  `LOOP.id/tag.DO/parallel/threadPool/BREAK`、`IF.id.ELIF/ELSE`、
  `SWITCH.tag.TO/DEFAULT`、`CATCH.tag.DO` 的 Java 动态类型语义。
- 带属性且包含 FINALLY 的 THEN 使用 maxWait 时，id/tag/bind 保留在 timeout
  内部原 ThenCondition，FINALLY 仍提升到 timeout 外层，符合 Java
  `MaxWaitTimeOperator#handleFinally`。
- 新增运行测试真实验证属性 When 的 ignoreError、属性 Loop 的循环次数、属性
  IF 的 ELSE 分支和属性 Catch 的 DO 处理器。最新 core 覆盖率为 region 67.22%、
  function 73.97%、line 69.70%，仍非 100%。
- `ParallelOperator` 现严格接受一个 Boolean：`parallel(true)` 启用循环并行，
  `parallel(false)` 保持串行，数字参数构建失败。循环 AST、For/While/Iterator
  Condition 构造器和 `LoopCondition#setParallel/isParallel` 共享同一个 `bool`
  状态；属性 Mods 后的 PARALLEL 仍按 Java 动态类型工作。
- 上述切片通过定向测试与 core 全特性测试。最新未排除生产文件的 core 覆盖率为
  region 66.83%（19,933/29,827）、function 73.82%（2,921/3,957）、line
  69.38%（14,948/21,544），仍缺 6,596 行，未达到 100%。
- 多次 Condition bind 不再共享单一 override 布尔值；每次调用只更新当前 key 的
  覆盖状态，同 key 重绑会替换旧状态。真实执行测试验证仅清除 k1 与仅清除 k2
  两个方向，未开启 override 的另一 key 保留 Node 绑定。Node bind 的第四参数
  与 Java 一致被忽略。
- 完整 core 测试与 workspace/all-features 编译通过；最新 core 覆盖率为 region
  66.89%（19,958/29,836）、function 73.89%（2,924/3,957）、line 69.42%
  （14,957/21,547），仍缺 6,590 行。
- 子链属性操作现在保留 Java 动态类型：首个 tag 创建 ThenCondition，首个 bind
  创建 ChainBindWrapperCondition，之后的 tag/bind/id 继续作用于包装 Condition；
  未修饰子链直接执行共享 Chain，普通 Node 的 `.tag(...).id(...)` 仍拒绝。
- ThenCondition 会下传自身 Condition bind 数据；Then 与 ChainBind 包装对象的
  显式 ID 均进入 `Executable::id()`。真实子链执行测试覆盖两种操作顺序及反例。
- 最新 core 覆盖率为 region 66.73%（20,028/30,012）、function 73.66%
  （2,928/3,975）、line 69.22%（14,997/21,665），仍缺 6,668 行。
- Java 通过 `OperatorHelper.convert(..., Executable.class)` 约束的 TAG、DATA、
  BIND、RETRY、MAX_WAIT 现在统一拒绝布尔字面量调用方；循环条件显式支持的布尔
  字面量仍保留。最新 core 覆盖率为 region 66.49%（20,034/30,131）、function
  73.57%（2,928/3,980）、line 69.00%（15,001/21,741），仍缺 6,740 行。
- Java `DataOperator` 对 Chain 不是创建一次性执行包装，而是通过
  `LiteflowMetaOperator#getNodes` 递归取得真实共享 Node 并调用 `setCmpData`。
  Rust 现以 `Chain` 内共享覆盖值、`Executable::apply_chain_cmp_data` 的容器递归
  和 `Frame` 传播表达同一可观测语义：父链引用、嵌套子链以及子链之后独立执行
  均读取新值，后一次赋值覆盖前一次；独立无 DATA Chain 仍读取自身 Node 数据。
- 新增真实运行测试覆盖三层子链递归、独立执行、末次写入与组件复用隔离；
  `cargo test -p liteflow-core` 和 `cargo check --workspace` 通过。最新
  core/all-features 覆盖率为 region 81.13%（20,226/24,931）、function
  83.02%（2,943/3,545）、line 81.93%（15,142/18,482），仍缺 3,340 行，
  不能标记为 100%。
- DATA 的 AST 测试现覆盖 Java Condition 可执行分组对应的全部 Rust 形态，
  `data_operator.rs` 行覆盖率为 95.77%；Chain 自定义克隆会复制 DATA 当前值但
  创建独立锁，规则热更新的新定义清理条件列表时不会改变旧发布快照。定向快照
  测试、完整 core 测试和 workspace 编译均通过。
- 本次修改文件已通过独立 `rustfmt --check`。全工作区 `cargo fmt --all -- --check`
  会进入外部 `vernal-framework`，并被其缺失的
  `crates/vernal-tonic/src/tonic_aop_error_mapper.rs` 阻断，不能记作 LiteFlow
  格式失败。编译/测试通过仍不能替代外部服务生产验证。
- `aop/AspectHolder` 经调用面与 Java 对象复核判定为重复伪对象：真实 Java
  `CmpAroundAspectHolder` 对应 `spi/holder/cmp_around_aspect_holder.rs`，业务
  `ICmpAroundAspect` 列表则由每个 `FlowBus` 隔离保存并在构建节点时生成执行
  快照。删除前者不改变任何执行路径；相关 SPI fallback/clean、脚本四阶段回调、
  节点成功/失败顺序及全局业务切面定向测试全部通过。
- Rust 公共 `RuleSourceWatcher` 现有真实异步测试覆盖首次加载、显式 reload、
  Chain 删除对账、Script 节点卸载、轮询相同指纹、读取失败、解析失败、恢复后
  发布新版本及 `JoinHandle::abort`。失败分支不推进指纹，也不删除上次成功管理的
  Chain；下一轮仍会重试同一新版本。三种 `RuleFormat` 均走真实 parser。
- 最新 core/all-features 覆盖率为 region 81.71%（20,359/24,915）、function
  83.42%（2,954/3,541）、line 82.43%（15,224/18,468），未达到 100%。
- `ScriptValidator` 不再以 Rust `language -> ValidationResp` Map 扩展替代 Java
  重载：新增 `ScriptTypeEnum` 强类型校验/异常响应和
  `Map<ScriptTypeEnum,String> -> boolean` 的遇错短路入口。定向测试通过真实
  `ScriptExecutorFactory` 构建器覆盖单 Rhai、多语言歧义、未知语言、自定义语言
  编译成功/失败、批量诊断和严格错误传播；该文件行覆盖由 12.07% 提升到 96%。
- 完整 core 回归与 workspace 编译通过；最新 core/all-features 覆盖率为 region
  82.09%（20,468/24,934）、function 83.72%（2,968/3,545）、line 82.78%
  （15,302/18,485），仍有 3,183 行未覆盖，整体目标保持未完成。
- Rhai 执行桥现在由真实脚本覆盖 `script_data_get/has/set` 的上下文读取、请求写集
  优先与回写，`script_context_call` 的请求级 ScriptBean、全局 fallback 和 serde
  JavaBean（含 `URL` acronym），以及 Kotlin `toInt`、Aviator 动态时间。负向
  路径覆盖缺失 Bean、非对象上下文、setter/getter 参数数量、边界外方法、数值/
  字符串/类型转换失败和普通 Rhai 求值错误；一参/两参 `liteflow_throw` 及函数
  包装均恢复原 LiteFlowException message/code。
- 该对象完整覆盖率现为 region 93.31%、function 83.02%、line 92.84%；该轮
  core/all-features 总覆盖率为 region 82.86%（20,661/24,934）、function
  84.40%（2,992/3,545）、line 83.68%（15,468/18,485），未达到 100%。
- `LiteFlowChainELBuilder` 的 Java 状态机与 AST 递归已完成新一轮实证：setter
  空值/既有 Chain、立即与两阶段编译、循环依赖、route 最终对象类型、全部
  Condition 构建形态、嵌套子链依赖收集、声明式方法/PROCESS 校验、Condition
  bind override 清理，以及 validateWithEx 深度优先缺失对象诊断均有对象级测试。
  类型化 AST 构建失败不会注册半成品，也不会污染下一次构建；实例编号快照会忽略
  缺字段 DTO 和其他 Chain 的记录，同时恢复同文件有效项且不重新生成。该对象行
  覆盖为 97.83%（855/874）；该轮 core/all-features 总覆盖率为 region 84.06%
  （20,960/24,934）、function 85.11%（3,017/3,545）、line 84.54%
  （15,627/18,485），仍有 2,858 行未覆盖，不能标记整体完成。
- `ParserHelper` 新增 JSON/XML 负向合同和延迟计划递归物化测试：Java 旧
  `condition[]` 到 EL 的兼容转换、route/body 约束、禁用/重复/空/截断输入、
  未知 XML 元素跳过、全部 Condition 引用收集，以及缺失/抽象 Chain、引用环和
  继承环都进入真实 Builder。该对象 line 93.32%（601/644）；该轮 core
  region 84.67%（21,112/24,934）、function 85.33%（3,025/3,545）、line
  85.06%（15,723/18,485），仍有 2,762 行未覆盖。
- `LiteflowConfig` 的 Java 命名访问器、Rust 兼容别名、Map/Agent/Duration 和
  空白类名默认回退已用单一状态对象完整往返，文件 region/function/line 均为
  100%。该轮 core region 85.36%（21,442/25,119）、function 86.41%
  （3,064/3,546）、line 85.74%（15,913/18,560），仍有 2,647 行未覆盖。
- `FlowBus` 对齐 Java v2.16.0 的重载与缓存边界：缺失 Chain 的
  `reloadChain` 会创建对象，两参数重载保留旧 route，三参数重载可替换 route；
  `reloadScript` 对缺失/非脚本节点静默返回；`cleanScriptCache` 仅清执行器编译
  缓存并保留 nodeMap；`unloadScriptNode` 在缓存已清空后仍删除脚本节点。
  注册表/降级节点/匿名 EL/规则刷新/异步执行和七种脚本 NodeType 均有真实测试。
  `flow_bus.rs` line 96.18%（630/655）、function 97.20%（104/107）；最新 core
  region 85.80%（21,558/25,125）、function 86.80%（3,078/3,546）、line
  86.22%（16,006/18,564），仍有 2,558 行未覆盖。
- `LiteflowMetaOperator` 新增 Java 三参数 `reloadOneChain` 对等入口，并验证
  getNodes 对 THEN/AND/OR/WHEN/IF/ELIF/ELSE/SWITCH/FOR/WHILE/ITERATOR/
  CATCH/NOT/PRE/FINALLY/属性包装的递归顺序，以及查询、全量刷新、批量卸载和
  route 热刷新。对象 function 100%（29/29）、line 99.48%（190/191）；
  最新 core region 86.32%（21,695/25,132）、function 87.03%
  （3,087/3,547）、line 86.70%（16,102/18,573），仍有 2,471 行未覆盖。
- `LiteFlowNodeBuilder#checkBuild` 现在与 Java 一样按 id、type 顺序聚合全部
  前置错误；十个静态工厂、NodePropBean、普通/脚本注册、既有节点、降级/非法
  类型、空脚本与文件失败均有对象级测试。该文件 line 94.57%（209/221）；
  最新 core region 86.67%（21,793/25,144）、function 87.33%
  （3,096/3,545）、line 87.04%（16,171/18,579），仍有 2,408 行未覆盖。
- `FlowExecutor#doExecute/doExecuteWithRoute` 已接回首次执行解析门闩，并以当前
  执行器 ParseMode 限定 Rust 多 FlowBus 的初始化作用域；Chain 缓存只在
  `PARSE_ONE_ON_FIRST_EXEC` 生效且容量必须大于零，空规则路径列表返回配置错误。
  首次主体/路由、失败响应、Future、混合 Parser、MonitorFile、route 异常、
  匿名 EL 与 timeout 均有真实测试。对象 line 98.62%（645/654）；最新 core
  region 87.27%（22,016/25,227）、function 87.68%（3,111/3,548）、line
  87.64%（16,332/18,636），仍有 2,304 行未覆盖。
- `ExecutorHelper#clearExecutorServiceMap` 已恢复 Java 的纯缓存清理语义，配置
  更新的安全停机另走私有路径；默认 60 秒和显式 timeout 两个 shutdown 重载均
  落地。WHEN/Hash/Main、Condition/Chain/Global 选择、配置与并发单例隔离均有
  真实测试。对象 line 94.27%（214/227）；最新 core region 87.48%
  （22,077/25,238）、function 87.83%（3,119/3,551）、line 87.92%
  （16,394/18,647），仍有 2,253 行未覆盖。
- `Node#execute` 已把 `isAccess` 从 `execute_once`/重试循环移到 NodeExecutor
  之前；AND/OR 和非 ALL WHEN 通过 `Executable#set_access_result` 复用预过滤
  true，ALL 仍保留 Java“策略层不预过滤、Node 自己判断”的特例。访问阶段错误按
  ChainEnd、continue-on-error、原错误顺序处理。`Frame` 同时保留 Rust 原生循环
  帧和 Java 两套独立循环兼容栈，解决连续 remove 误弹父层；空白 cmpData/bind
  按 `StrUtil.isBlank` 返回缺失并允许 Condition 回退。完整 core 与 workspace
  检查通过；全量采集并合并 NodeComponent 定向测试后的最新无排除 core
  覆盖率为 region 88.78%（22,469/25,308）、function 89.27%
  （3,178/3,560）、line 89.30%（16,705/18,707），仍有 2,002 行未覆盖。
  NodeComponent 已以默认方法、转换错误、空表达式、私有投递 fallback 和非法
  setter 补到 line 95.93%（330/344）；Node 的元数据、Slot API、监控三终态、
  isEnd 错误优先级及实例/类名两种执行器选择已补到 line/function 100%。
- `IteratorCondition` 已直接验证数组类型约束、串行/并行、两种 BREAK、未知线程池、
  Java 状态访问器和强类型分组替换；对象 function 100%、line 99.38%
  （160/161），不再仅依赖 EL 间接测试。
- `WhileCondition`、`ForCondition`、`SwitchCondition` 已按 Java v2.16.0
  完成对象级复核：WHILE 保留循环前单次 isAccess 与逐轮条件执行；FOR 仅接受
  Java Integer 对等整数，拒绝字符串/浮点并让负数执行零次；SWITCH 完整保留
  ID/tag/default、空白/Null、无目标和 PRE/FINALLY 禁令，其中空白判断已修正为
  `StrUtil.isNotBlank` 对等语义。三者 function/line 均为 100%；ITERATOR
  function 100%、line 99.39%（164/165）。
- `cargo test -p liteflow-core` 与 `cargo check -p liteflow-core` 通过。最新无
  生产文件排除的全量覆盖率为 region 89.98%（22,786/25,322）、function
  90.14%（3,208/3,559）、line 90.45%（16,925/18,711），仍缺 1,786 行，
  因此 100% 总门禁继续未通过。`cargo check --workspace` 当前仅被外部路径依赖
  `vernal-framework` 的既有 Rust 语法错误阻断。
- `IfCondition` 已恢复 true 分支可空及命中时 `NoIfTrueNode` 的 Java 运行期
  合同；ELIF 依照 Java `ElifOperator` 构成的嵌套 IF，在判定项
  `isAccess=false` 时结束内层 IF，不会错误继续 ELSE。对象 function 100%、
  line 97.18%（138/142），完整 core 回归通过。最新全量覆盖率为 region
  90.37%（22,894/25,334）、function 90.31%（3,215/3,560）、line 90.73%
  （16,984/18,720），仍缺 1,736 行。

## 十五、2026-08-04 测试补齐与证据更新

- 新增 20 个外置测试文件（约 130 个对象级测试），覆盖 Slot/Frame/CmpContext、
  LiteflowResponse/CmpStep 全 API、Decl 生命周期错误路径、Condition 分组协议
  遍历（12 类）、FlowBus 注册/匿名链 MD5/脚本卸载重载缓存、ParserHelper
  JSON/XML 负向与抽象链/循环检测、MonitorFile 生命周期、ScriptKind/
  NodeTypeEnum 枚举、CopyOnWriteHashMap/LimitQueue、脚本组件钩子、
  FlowExecutor future 入口、ScriptExecuteWrap 快照、SerialsUtil、并行 spawnAll。
- 名称收口：`NacosParserHelper`、`elsql_exception.rs`、`re_act_*` 三文件按
  命名规则修正；8 个 Spring→Vernal 对象登记为正式批准例外；15 个
  Janino/JSR223 对象逐项裁决完成（10 项有 Rust 等价入口、5 项 JVM 专有）。
- 最新 core 全特性覆盖率：region 93.59%、function 94.07%、line 93.99%
  （对比 2026-07-29 基线 line 90.68% 提升 3.31pp）。剩余缺口包含系统性
  LLVM 行归因噪声（catch_condition 第 104 行、liteflow_response `new`、
  slot `set_response_data` 均有断言证实执行但报告未覆盖）与私有/防御分支，
  不能以伪造调用或排除生产文件达成 95% 门禁。
- `cargo check --workspace --all-features --all-targets` 0 error；
  `cargo test -p liteflow-core --all-features` 全绿；新增测试文件 clippy
  0 警告；`liteflow-agent-provider-core` 0 警告（Windows icacls 平台条件
  代码、Codex 生产路径复用 resolve_instructions）。
