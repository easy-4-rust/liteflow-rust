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
//! use liteflow_rust::{FlowBus, cmp};
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

pub mod builder;
pub mod core;
pub mod el;
pub mod enums;
pub mod exception;
pub mod flow;
pub mod parser;
pub mod util;
pub mod lifecycle;
pub mod monitor;
pub mod aop;
pub mod instance_id;
pub mod rule_plugin;
pub mod script;
pub mod slot;

// ---------- 顶层便捷导出 ----------
pub use core::{cmp, FlowExecutor, FnComponent, NodeComponent};
pub use el::{parse_el, El, Mods, NodeRef, WhenOpts};
pub use enums::{CmpStepTypeEnum, ConditionTypeEnum, ParallelStrategyEnum};
pub use exception::{LFResult, LiteflowError};
pub use flow::entity::cmp_step::CmpStep;
pub use flow::{FlowBus, LiteflowResponse};
pub use script::script_component::{ScriptComponent, ScriptKind};
pub use slot::{CmpContext, Ctx, Frame, Slot};

/// 步骤类型别名（兼容旧 API）
pub type CmpStepType = CmpStepTypeEnum;

/// 规则加载便捷入口（对应 parser 包）
pub mod rule {
    pub use crate::parser::local_json_flow_el_parser::{load_json_file, load_json_str};
    pub use crate::parser::local_xml_flow_el_parser::{load_xml_file, load_xml_str};
    pub use crate::parser::local_yml_flow_el_parser::{load_yml_file, load_yml_str};
    pub use crate::parser::monitor_file::RuleWatcher;
}
