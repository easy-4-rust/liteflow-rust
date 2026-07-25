//! 对应 Java 类：com.yomahub.liteflow.flow.id.RequestIdGenerator
//!
//! Id 生成接口。

/// 对应 RequestIdGenerator
pub trait RequestIdGenerator: Send + Sync {
    /// 对应 generate()：获取唯一 id
    fn generate(&self) -> String;
}
