//! 对应 flow.parallel.strategy 包。

pub mod all_of_parallel_executor;
pub mod any_of_parallel_executor;
mod parallel_opts;
mod parallel_outcome;
mod parallel_strategy_executor;
pub mod parallel_strategy_helper;
mod parallel_strategy_support;
pub mod percentage_of_parallel_executor;
pub mod specify_parallel_executor;

pub use parallel_opts::ParallelOpts;
pub use parallel_outcome::ParallelOutcome;
pub use parallel_strategy_executor::ParallelStrategyExecutor;
pub use parallel_strategy_helper::ParallelStrategyHelper;
pub use parallel_strategy_support::{collect, spawn_all};
