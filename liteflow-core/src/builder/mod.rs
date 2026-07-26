//! 对应 builder 包。

pub mod el;
pub mod lite_flow_node_builder;
pub mod prop;

pub use lite_flow_node_builder::LiteFlowNodeBuilder;
pub use prop::{ChainPropBean, NodePropBean};
