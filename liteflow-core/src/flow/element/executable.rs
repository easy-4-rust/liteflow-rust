//! 对应 flow.element.Executable 接口。

use crate::exception::LFResult;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;

/// 所有可执行元素（Node / 各 Condition / Chain）的统一接口
#[async_trait]
pub trait Executable: Send + Sync {
    /// execute(slotIndex)
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value>;
    /// getId()（节点返回 id，条件返回类型名）
    fn id(&self) -> &str {
        ""
    }
    /// getTag()
    fn tag(&self) -> Option<&str> {
        None
    }
    /// 是否为 PRE / FINALLY（IfCondition、SwitchCondition 的目标校验用）
    fn is_pre_or_finally(&self) -> bool {
        false
    }
    /// isAccess(slotIndex)（2.16：AND/OR 在求值前按 isAccess 过滤子项，
    /// isAccess 异常等同于不可访问被排除；Condition 默认 true，Node 委托组件）
    async fn is_access(&self, _ctx: &Ctx, _frame: &Frame) -> bool {
        true
    }
}
