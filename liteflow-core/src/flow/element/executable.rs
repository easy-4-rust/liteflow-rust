//! 对应 flow.element.Executable 接口。

use crate::enums::ExecuteableTypeEnum;
use crate::exception::LFResult;
use crate::slot::{Ctx, Frame};
use async_trait::async_trait;
use serde_json::Value;

/// 所有可执行元素（Node / 各 Condition / Chain）的统一接口
#[async_trait]
pub trait Executable: Send + Sync {
    /// execute(slotIndex)
    async fn execute(&self, ctx: &Ctx, frame: &Frame) -> LFResult<Value>;

    /// 返回统一可执行对象类型。对应 Java `Executable#getExecuteType()`。
    ///
    /// Rust 中绝大多数 `Executable` 实现是 Condition，因此默认返回 Condition；
    /// Node 与 Chain 分别覆盖为对应类型。
    fn execute_type(&self) -> ExecuteableTypeEnum {
        ExecuteableTypeEnum::Condition
    }

    /// 按当前对象的结构遍历顺序收集其包含的全部 Node ID。
    ///
    /// Node 返回自身；Condition 与 Chain 分别覆盖为递归遍历。默认实现仍兼容
    /// 测试或扩展代码中仅通过 `execute_type` 声明为 Node 的轻量对象。
    /// 对应 Java: `Condition#getAllNodeInCondition` 的 Node 分支。
    #[must_use]
    fn collect_node_ids(&self) -> Vec<String> {
        if self.execute_type() == ExecuteableTypeEnum::Node && !self.id().is_empty() {
            vec![self.id().to_string()]
        } else {
            Vec::new()
        }
    }

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
