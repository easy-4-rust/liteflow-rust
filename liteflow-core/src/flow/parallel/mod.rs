//! 对应 flow.parallel 包。

pub mod strategy;
pub mod completable_future_timeout;
pub mod parallel_supplier;
pub mod when_future_obj;

pub use completable_future_timeout::complete_on_timeout;
pub use parallel_supplier::ParallelSupplier;
pub use when_future_obj::WhenFutureObj;
