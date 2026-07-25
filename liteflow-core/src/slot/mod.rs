//! 对应 liteflow-core slot 包。

pub mod slot;
pub mod databus;
pub mod default_context;

pub use databus::{gen_request_id, Ctx, Frame};
pub use default_context::CmpContext;
pub use slot::Slot;
