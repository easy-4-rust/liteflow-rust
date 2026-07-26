//! LiteFlow EL 链式构建器。
//!
//! 对应 Java 模块：`liteflow-el-builder`。每个 Java 对象均迁移到同名语义的
//! snake_case 文件，输出既支持 Java 风格完整语句，也支持 Rust 运行时可直接解析的表达式。

pub mod builder;

pub use builder::{
    AndELWrapper, BoxedELWrapper, CatchELWrapper, CommonNodeELWrapper, ELBuilderError,
    ELBuilderResult, ELBus, ELWrapper, FinallyELWrapper, IfELWrapper, IntoELWrapper, LoopELWrapper,
    NodeELWrapper, NotELWrapper, OrELWrapper, ParELWrapper, PreELWrapper, RenderMode, SerELWrapper,
    SwitchELWrapper, ThenELWrapper, WhenELWrapper, WrapperKind,
};
