//! 对应 Java 类：com.yomahub.liteflow.flow.element.condition.CatchCondition
//!
//! 捕获执行 DO；无 DO 则异常继续抛出。
//!
//! 差异说明：
//! - Java 在 catch item 为空时抛 CatchErrorException；Rust 端 catch_item 为
//!   非空字段（builder 保证），不存在该运行期分支。
//! - Java 通过 DataBus.getSlot(slotIndex).removeException() 清除 slot 异常；
//!   Rust 端直接复位 Slot.exception（pub 字段），语义一致。

use super::Condition;
use crate::enums::ConditionTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::executable::Executable;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct CatchCondition {
    catch_item: Arc<dyn Executable>,
    do_item: Option<Arc<dyn Executable>>,
}

impl CatchCondition {
    pub fn new(catch_item: Arc<dyn Executable>, do_item: Option<Arc<dyn Executable>>) -> Self {
        Self {
            catch_item,
            do_item,
        }
    }

    /// 对应 Java CatchCondition#getConditionType
    pub fn condition_type(&self) -> ConditionTypeEnum {
        ConditionTypeEnum::Catch
    }
}

#[async_trait]
impl Executable for CatchCondition {
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value> {
        match self.catch_item.execute(ctx, frame).await {
            Ok(v) => Ok(v),
            Err(LiteflowError::ChainEnd) => Err(LiteflowError::ChainEnd),
            Err(e) => {
                ctx.set_exception(&e.to_string());
                match &self.do_item {
                    Some(d) => {
                        let r = d.execute(ctx, frame).await;
                        if r.is_ok() {
                            // 对应 Java CatchCondition#executeCondition：
                            // catch 之后需要把 exception 清除掉——正如同 java 的 catch，
                            // 异常自己处理了属于正常流程，整个流程状态应该是成功的
                            if let Ok(mut ex) = ctx.inner.exception.lock() {
                                *ex = None;
                            }
                        }
                        r
                    }
                    None => Err(e),
                }
            }
        }
    }

    fn id(&self) -> &str {
        "CATCH"
    }
}

impl Condition for CatchCondition {
    fn condition_type(&self) -> ConditionTypeEnum {
        CatchCondition::condition_type(self)
    }
}
