//! 对应 Java `core.NodeComponent` 基类。
//!
//! 四个有返回值的 Java 子类已经拆到各自文件，并在适配到本 trait 时用
//! `serde_json::Value` 传递类型结果：
//! - 普通组件 → `Value::Null`
//! - 布尔组件（IF/WHILE/BREAK/AND/OR/NOT）→ `Value::Bool`
//! - SWITCH 组件 → `Value::String`（目标 id，可带 "id:tag"）
//! - FOR 组件 → 数字
//! - ITERATOR 组件 → `Value::Array`

use crate::enums::NodeTypeEnum;
use crate::exception::LiteflowError;
use crate::flow::executor::NodeExecutor;
use crate::slot::CmpContext;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[async_trait]
pub trait NodeComponent: Send + Sync + 'static {
    /// process() / processIf() / processSwitch() / processFor() / processIterator()
    async fn process(&self, ctx: &CmpContext) -> Result<Value, LiteflowError>;

    /// beforeProcess()
    async fn before_process(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }
    /// onSuccess()：`process` 成功后的回调。
    ///
    /// 对应 Java: `com.yomahub.liteflow.core.NodeComponent#onSuccess`。
    /// 回调抛错时按组件执行失败处理，并继续进入 `on_error` 与 `after_process`。
    async fn on_success(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }
    /// afterProcess()
    async fn after_process(&self, ctx: &CmpContext) {
        // 默认组件没有收尾副作用；实现方可覆盖该钩子。
        let _ = ctx;
    }
    /// onError()
    async fn on_error(&self, ctx: &CmpContext, error: &LiteflowError) {
        // 默认组件不吞掉也不改写错误；错误仍由 Node 执行主干传播。
        let _ = (ctx, error);
    }
    /// isAccess()
    fn is_access(&self, _ctx: &CmpContext) -> bool {
        true
    }
    /// isContinueOnError()
    fn is_continue_on_error(&self) -> bool {
        false
    }
    /// 是否需要失败补偿。
    ///
    /// Java 构造器通过反射判断组件是否覆盖 `rollback()`；Rust trait 无法在运行时
    /// 可靠判断默认方法是否被覆盖，因此显式返回该能力。后续 `liteflow-derive`
    /// 会在声明了 rollback 方法时自动生成此标记。
    fn is_rollback(&self) -> bool {
        false
    }
    /// Rollbackable.rollback()。
    ///
    /// 对应 Java: `com.yomahub.liteflow.core.NodeComponent#rollback`。
    async fn rollback(&self, _ctx: &CmpContext) -> Result<(), LiteflowError> {
        Ok(())
    }
    /// getName()
    fn name(&self) -> &str {
        ""
    }
    /// getNodeId()：返回初始化器写入的节点 id。
    fn node_id(&self) -> &str {
        ""
    }
    /// getType()：返回初始化器写入的节点类型。
    fn node_type(&self) -> Option<NodeTypeEnum> {
        None
    }
    /// getRetryCount()：最大重试次数（默认 0 = 不重试，总尝试次数 = retry_count + 1）
    fn retry_count(&self) -> usize {
        0
    }
    /// getRetryForExceptions() 语义：判断抛出的异常是否命中组件声明的可重试异常范围
    /// （Java 用 retryForExceptions 列表 + isAssignableFrom 判定，Rust 化为谓词方法）
    fn is_retry_for(&self, _e: &LiteflowError) -> bool {
        false
    }
    /// getNodeExecutorClass()：指定自定义节点执行器；None 表示使用 DefaultNodeExecutor
    /// （Java 返回 Class 由 NodeExecutorHelper 经 DI 容器实例化并缓存，
    /// Rust 端无 DI 容器，直接提供 Arc 实例）
    fn node_executor(&self) -> Option<Arc<dyn NodeExecutor>> {
        None
    }
}
