//! 对应 Java 包：com.yomahub.liteflow.flow.id
//!
//! requestId 生成体系：RequestIdGenerator 接口 + DefaultRequestIdGenerator
//! 默认实现 + IdGeneratorHolder 单例帮助器。

pub mod request_id_generator;
pub mod default_request_id_generator;
pub mod id_generator_holder;

pub use request_id_generator::RequestIdGenerator;
pub use default_request_id_generator::{fast_simple_uuid, DefaultRequestIdGenerator};
pub use id_generator_holder::IdGeneratorHolder;
