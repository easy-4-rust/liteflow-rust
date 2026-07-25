//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.ThenCondition
//!
//! 串行器：pre → 主体顺序执行 → finally（必执行）。
//! 异常记入 slot 并向上抛出；ChainEnd 原样上抛。
//!
//! 差异说明：
//! - Java 在 finally 块中执行 FinallyCondition，若 finally 自身抛异常会覆盖主异常上抛；
//!   Rust 端保留首个异常（finally 异常仅在主流程无异常时生效）。
//! - Java 按 isSubChain 区分 setException / setSubException；Rust 端 slot 无子链
//!   异常槽位，统一记 set_exception（见 crate::slot::Ctx）。

use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct ThenCondition {
    pre_list: Vec<Arc<dyn Executable>>,
    executable_list: Vec<Arc<dyn Executable>>,
    finally_list: Vec<Arc<dyn Executable>>,
}

impl ThenCondition {
    pub fn new() -> Self {
        Self { pre_list: Vec::new(), executable_list: Vec::new(), finally_list: Vec::new() }
    }
    /// 对应 Java ThenCondition#addExecutable：按类型分流，
    /// PreCondition → pre_list，FinallyCondition → finally_list，其余进主体列表
    pub fn add_executable(&mut self, item: Arc<dyn Executable>) {
        if item.is_pre_or_finally() {
            match item.id() {
                "PRE" => {
                    self.pre_list.push(item);
                    return;
                }
                "FINALLY" => {
                    self.finally_list.push(item);
                    return;
                }
                _ => {}
            }
        }
        self.executable_list.push(item);
    }
    /// 对应 Java ThenCondition#addPreCondition
    pub fn add_pre_condition(&mut self, item: Arc<dyn Executable>) {
        self.pre_list.push(item);
    }
    /// 对应 Java ThenCondition#addFinallyCondition
    pub fn add_finally_condition(&mut self, item: Arc<dyn Executable>) {
        self.finally_list.push(item);
    }
    /// 对应 Java ThenCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Then
    }
}

#[async_trait]
impl Executable for ThenCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        let mut err: Option<LiteflowError> = None;
        for item in self.pre_list.iter().chain(self.executable_list.iter()) {
            match item.execute(ctx, frame).await {
                Ok(_) => {}
                Err(e) => {
                    err = Some(e);
                    break;
                }
            }
        }
        // finally 必执行
        for fin in &self.finally_list {
            if let Err(fe) = fin.execute(ctx, frame).await {
                if err.is_none() {
                    err = Some(fe);
                }
            }
        }
        match err {
            Some(LiteflowError::ChainEnd) => Err(LiteflowError::ChainEnd),
            Some(e) => {
                ctx.set_exception(&e.to_string());
                Err(e)
            }
            None => Ok(Value::Null),
        }
    }

    fn id(&self) -> &str {
        "THEN"
    }
}
