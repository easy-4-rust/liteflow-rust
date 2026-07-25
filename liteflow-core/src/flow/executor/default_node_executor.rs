//! 对应 Java 类：com.yomahub.liteflow.flow.executor.DefaultNodeExecutor
//!
//! 默认的节点执行器：直接使用 NodeExecutor trait 的默认实现
//! （Java 中 execute() 覆写仅为透传 super.execute()，Rust 端无需覆写）。

use crate::flow::executor::node_executor::NodeExecutor;

/// 默认的节点执行器（对应 DefaultNodeExecutor）
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultNodeExecutor;

impl NodeExecutor for DefaultNodeExecutor {}
