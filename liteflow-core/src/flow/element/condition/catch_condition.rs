use std::sync::Arc;

use crate::{
    context::Context,
    flow::element::{Condition, Executable},
    slot::DataBus,
};

/// CATCH 条件
///
/// 捕获异常
///
/// 捕获异常的执行流程，相当于 try-catch 语句中的 catch 部分
///
/// 本类是 CATCH 条件的具体实现
#[derive(Clone)]
pub struct CatchCondition {
    /// 捕获异常的执行流程
    pub r#catch: Box<dyn Executable>,
    /// 错误处理流程
    pub r#do: Option<Box<dyn Executable>>,
}

#[async_trait::async_trait]
impl Condition for CatchCondition {
    /// 执行捕获异常
    ///
    /// 如果捕获异常发生，则执行错误处理流程
    async fn execute(&self, slot_key: usize) -> anyhow::Result<()> {
        match self.r#catch.execute(slot_key).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // 捕获异常发生，将异常信息设置到插槽中
                let slot = DataBus::get_slot(slot_key).expect("slot not found");
                slot.set_exception(e);
                // 执行错误处理流程
                if let Some(r#do) = &self.r#do {
                    r#do.execute(slot_key).await
                } else {
                    Ok(())
                }
            }
        }
    }

    /// 获取捕获异常的执行流程的 id
    fn id(&self) -> String {
        self.r#catch.id()
    }

    /// 获取捕获异常的执行流程的标签
    fn tag(&self) -> String {
        self.r#catch.tag()
    }
}

impl std::fmt::Debug for CatchCondition {
    /// 打印捕获异常的执行流程
    ///
    /// 打印捕获异常的执行流程和错误处理流程
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CatchCondition")
            .field("catch", &self.r#catch.id())
            .field("do", &self.r#do.as_ref().map(|d| d.id()))
            .finish()
    }
}
