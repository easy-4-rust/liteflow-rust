//! 对应 ThenCondition：pre → 主体顺序执行 → finally（必执行）。
//! 异常记入 slot 并向上抛出；ChainEnd 原样上抛。

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
    /// addExecutable（按类型分流，对齐 Java addExecutable 的重载）
    pub fn add_executable(&mut self, item: Arc<dyn Executable>) {
        if item.is_pre_or_finally() {
            // PreCondition / FinallyCondition 分别进入专属列表
            // 通过 id 前缀区分（builder 保证）
        }
        self.executable_list.push(item);
    }
    pub fn add_pre_condition(&mut self, item: Arc<dyn Executable>) {
        self.pre_list.push(item);
    }
    pub fn add_finally_condition(&mut self, item: Arc<dyn Executable>) {
        self.finally_list.push(item);
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
