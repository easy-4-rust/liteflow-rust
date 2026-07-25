//! 对应 flow.element.condition 包：编排语义的执行体。

use serde_json::Value;

use crate::el::WhenOpts;
use crate::exception::LiteflowError;

pub mod then_condition;
pub mod when_condition;
pub mod if_condition;
pub mod switch_condition;
pub mod loop_condition;
pub mod for_condition;
pub mod while_condition;
pub mod iterator_condition;
pub mod catch_condition;
pub mod and_or_condition;
pub mod not_condition;
pub mod retry_condition;
pub mod timeout_condition;
pub mod ignore_error_condition;
pub mod pre_condition;
pub mod finally_condition;
pub mod chain_bind_wrapper_condition;
pub mod bind_wrapper_condition;

pub use then_condition::ThenCondition;
pub use when_condition::WhenCondition;
pub use if_condition::IfCondition;
pub use switch_condition::SwitchCondition;
pub use loop_condition::LoopCondition;
pub use for_condition::ForCondition;
pub use while_condition::WhileCondition;
pub use iterator_condition::IteratorCondition;
pub use catch_condition::CatchCondition;
pub use and_or_condition::{AndOrCondition, BooleanConditionTypeEnum};
pub use not_condition::NotCondition;
pub use retry_condition::RetryCondition;
pub use timeout_condition::TimeoutCondition;
pub use ignore_error_condition::IgnoreErrorCondition;
pub use pre_condition::PreCondition;
pub use finally_condition::FinallyCondition;
pub use chain_bind_wrapper_condition::ChainBindWrapperCondition;
pub use bind_wrapper_condition::BindWrapperCondition;

/// WHEN 参数统一解析（对应 WhenELResolver 的 opts 语义）
pub fn parse_when_opts(opts: &WhenOpts) -> WhenParams {
    WhenParams {
        any: opts.any,
        must: opts.must,
        percentage: opts.percentage,
        parallel_strategy: opts.parallel_strategy,
        parallel_group: opts.parallel_group.clone(),
        thread_pool: opts.thread_pool.clone(),
        ignore_error: opts.ignore_error,
        max_wait_ms: opts.max_wait_ms,
    }
}

#[derive(Debug, Default, Clone)]
pub struct WhenParams {
    pub any: bool,
    pub must: bool,
    pub percentage: Option<f64>,
    pub parallel_strategy: Option<String>,
    pub parallel_group: Option<String>,
    pub thread_pool: Option<String>,
    pub ignore_error: bool,
    pub max_wait_ms: Option<u64>,
}

/// 布尔节点返回值校验（对应 BooleanNode 的类型约束语义）
pub fn expect_bool(id: &str, v: &Value) -> Result<bool, LiteflowError> {
    v.as_bool().ok_or_else(|| LiteflowError::NodeTypeError {
        node: id.to_string(),
        expect: "Boolean",
    })
}
