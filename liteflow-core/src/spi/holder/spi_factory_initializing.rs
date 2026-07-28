//! Java `SpiFactoryInitializing` 的 Rust 映射。

use super::{
    CmpAroundAspectHolder, ContextAwareHolder, ContextCmpInitHolder, DeclComponentParserHolder,
    LiteflowComponentSupportHolder, PathContentParserHolder,
};

/// 统一预加载或清理所有 LiteFlow SPI Holder。
///
/// 对应 Java: `com.yomahub.liteflow.spi.holder.SpiFactoryInitializing`。
pub struct SpiFactoryInitializing;

impl SpiFactoryInitializing {
    /// 清理全部 SPI Holder，使下次访问重新选择本地或宿主实现。
    ///
    /// 对应 Java: `SpiFactoryInitializing#clean`。
    pub fn clean() {
        CmpAroundAspectHolder::clean();
        ContextAwareHolder::clean();
        ContextCmpInitHolder::clean();
        DeclComponentParserHolder::clean();
        LiteflowComponentSupportHolder::clean();
        PathContentParserHolder::clean();
    }

    /// 预加载全部 SPI Holder。
    ///
    /// 对应 Java: `SpiFactoryInitializing#loadInit`。
    pub fn load_init() {
        let _ = CmpAroundAspectHolder::load_cmp_around_aspect();
        let _ = ContextAwareHolder::load_context_aware();
        let _ = ContextCmpInitHolder::load_context_cmp_init();
        let _ = DeclComponentParserHolder::load_decl_component_parser();
        let _ = LiteflowComponentSupportHolder::load_liteflow_component_support();
        let _ = PathContentParserHolder::load_context_aware();
    }
}
