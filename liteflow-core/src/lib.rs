//! # liteflow-rust
//!
//! [LiteFlow](https://github.com/dromara/liteflow)（dromara 组件式规则引擎）的
//! Rust 语义移植版。包结构、对象模型与 Java 版同构：
//!
//! - `core` — NodeComponent / FlowExecutor
//! - `flow` — FlowBus / Chain / Node / LiteflowResponse / CmpStep
//! - `flow.element.condition` — 14 种 Condition 对象（每种一个文件）
//! - `flow.parallel.strategy` — AllOf / AnyOf / PercentageOf / Specify 并行策略执行器
//! - `slot` — Slot / DataBus / DefaultContext
//! - `builder.el` — LiteFlowChainELBuilder（El 语法树 → Condition 对象树）
//! - `parser` — LocalJsonFlowELParser / MonitorFile 热刷新
//! - `el` — EL 词法/语法解析（对应 Java 版底层的 QLExpress 层）
//!
//! ```no_run
//! use liteflow_core::{FlowBus, cmp};
//! use serde_json::Value;
//!
//! #[tokio::main]
//! async fn main() {
//!     let bus = FlowBus::new();
//!     bus.register("a", cmp(|ctx| async move {
//!         ctx.set_data("x", Value::from(1));
//!         Ok(Value::Null)
//!     }));
//!     bus.register("b", cmp(|_ctx| async move { Ok(Value::Null) }));
//!     bus.add_chain("chain1", "THEN(a, b)").unwrap();
//!     let resp = bus.execute("chain1").await;
//!     assert!(resp.is_success());
//! }
//! ```

pub mod aop;
pub mod builder;
pub mod common;
pub mod core;
pub mod el;
pub mod enums;
pub mod exception;
pub mod flow;
pub mod lifecycle;
pub mod log;
pub mod meta;
pub mod monitor;
pub mod parser;
pub mod rule_plugin;
pub mod script;
pub mod slot;
pub mod spi;
pub mod thread;
pub mod util;

// ---------- 顶层便捷导出 ----------
/// 供 `liteflow-derive` 生成代码使用，调用方无需重复声明 async-trait 依赖。
pub use async_trait::async_trait;
pub use builder::{ChainPropBean, LiteFlowNodeBuilder, NodePropBean};
pub use common::{ChainConstant, LocalDefaultFlowConstant};
pub use core::execute_option::{ExecuteOption, gen_conversation_id};
pub use core::{
    ComponentInitializer, FlowExecutor, FlowExecutorHolder, FlowInitHook, FnComponent,
    NodeBooleanComponent, NodeComponent, NodeForComponent, NodeIteratorComponent,
    NodeSwitchComponent, cmp,
};
pub use el::{El, Mods, NodeRef, WhenOpts, parse_el};
pub use enums::{
    ChainExecuteModeEnum, CmpStepTypeEnum, ConditionTypeEnum, ExecuteableTypeEnum,
    FlowParserTypeEnum, InnerChainTypeEnum, NodeTypeEnum, ParallelStrategyEnum, ParseModeEnum,
};
pub use exception::{LFResult, LiteflowError};
pub use flow::element::condition::abstract_condition::AbstractCondition;
pub use flow::element::{Condition, Rollbackable};
pub use flow::entity::InstanceInfoDto;
pub use flow::entity::cmp_step::CmpStep;
pub use flow::id::{DefaultRequestIdGenerator, IdGeneratorHolder, RequestIdGenerator};
pub use flow::parallel::LoopFutureObj;
pub use flow::parallel::strategy::ParallelStrategyHelper;
pub use flow::{FlowBus, LiteflowResponse};
pub use flow::{FlowEvent, FlowEventBuilder, FlowEventListener, FlowEventPublisher, listener};
pub use lifecycle::{
    LifeCycle, LifeCycleHolder, PostProcessChainBuildLifeCycle, PostProcessChainExecuteLifeCycle,
    PostProcessFlowExecuteLifeCycle, PostProcessNodeBuildLifeCycle,
    PostProcessScriptEngineInitLifeCycle,
};
pub use monitor::{CompStatistics, MonitorBus, MonitorTimeTask};
pub use script::{
    ScriptBooleanComponent, ScriptCommonComponent, ScriptComponent, ScriptForComponent,
    ScriptIteratorComponent, ScriptKind, ScriptSwitchComponent,
};
/// 供 `liteflow-derive` 生成代码使用，调用方无需重复声明 serde_json 依赖。
pub use serde_json;
pub use slot::{CmpContext, Ctx, DataBus, Frame, Slot};
pub use spi::{
    CmpAroundAspectHolder, ContextAware, ContextAwareHolder, ContextCmpInit, DeclComponentParser,
    DeclComponentParserHolder, LiteflowComponentSupport, LocalDeclComponentParser,
    PathContentParser, SpiFactoryCleaner, SpiFactoryInitializing, SpiPriority,
};
pub use thread::{
    ExecutorBuilder, ExecutorCondition, ExecutorConditionBuilder, ExecutorHelper, ExecutorService,
    LiteFlowDefaultGlobalExecutorBuilder, LiteFlowDefaultMainExecutorBuilder,
};
pub use util::{
    BoundedPriorityBlockingQueue, ConversationIdGenerator, JsonUtil, LimitQueue, PathMatchUtil,
    SelectiveJavaEscaper, TupleOf2, TupleOf3,
};

/// 步骤类型别名（兼容旧 API）
pub type CmpStepType = CmpStepTypeEnum;

/// 规则加载便捷入口（对应 parser 包）
pub mod rule {
    pub use crate::parser::el::{
        load_json_file, load_json_str, load_xml_file, load_xml_str, load_yml_file, load_yml_str,
    };
    pub use crate::parser::monitor_file::RuleWatcher;
}
