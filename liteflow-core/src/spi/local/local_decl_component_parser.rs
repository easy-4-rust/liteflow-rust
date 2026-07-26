//! Java `LocalDeclComponentParser` 的 Rust 映射。

use crate::core::proxy::{DeclWarpBean, LiteFlowProxyUtil};
use crate::exception::{LFResult, LiteflowError};
use crate::spi::{DeclComponentParser, SpiPriority};

/// 非容器环境的声明式组件解析器。
///
/// Java 非 Spring 环境无法反射创建声明式代理，因此直接抛
/// `NotSupportDeclException`；Rust 的 `liteflow-derive` 已在编译期生成完整元数据，
/// 本地解析器可以安全地校验并透传该元数据，不依赖 Spring 或 Vernal。
///
/// 对应 Java: `com.yomahub.liteflow.spi.local.LocalDeclComponentParser`。
#[derive(Debug, Default)]
pub struct LocalDeclComponentParser;

impl LocalDeclComponentParser {
    /// 创建本地声明式组件解析器。
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl DeclComponentParser for LocalDeclComponentParser {
    /// 校验过程宏生成的声明式元数据并返回单个包装对象。
    ///
    /// 对应 Java: `LocalDeclComponentParser#parseDeclBean(Class)`；Rust 编译期元数据
    /// 使非容器环境具备了 Java 本地实现缺失的安全解析能力。
    fn parse_decl_bean(&self, decl_warp_bean: DeclWarpBean) -> LFResult<Vec<DeclWarpBean>> {
        if !LiteFlowProxyUtil::is_declare_cmp(&decl_warp_bean) {
            return Err(LiteflowError::NotSupportDecl(format!(
                "type[{}] does not contain liteflow declaration metadata",
                decl_warp_bean.raw_clazz()
            )));
        }
        Ok(vec![decl_warp_bean])
    }
}

impl SpiPriority for LocalDeclComponentParser {
    /// 返回本地实现优先级 2。对应 Java: `LocalDeclComponentParser#priority`。
    fn priority(&self) -> i32 {
        2
    }
}
