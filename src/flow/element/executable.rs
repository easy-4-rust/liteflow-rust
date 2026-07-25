//! 对应 flow.element.Executable：Node 与 Condition 的统一执行协议。

use crate::exception::LFResult;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Executable: Send + Sync {
    /// 执行（对应 execute(slotIndex)）
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value>;
    /// 标识（节点 id / 条件类型名）
    fn id(&self) -> &str;
    /// 标签（Node.tag，Condition 默认 None）
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
