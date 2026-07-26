//! Java `DeclComponentParser` 的 Rust 映射。

use crate::core::proxy::DeclWarpBean;
use crate::exception::LFResult;

use super::SpiPriority;

/// 声明式组件解析器接口。
///
/// Java 输入 `Class<?>` 并通过反射生成 `DeclWarpBean`；Rust 过程宏已经完成反射
/// 阶段，因此 SPI 接收编译期生成的包装对象，并允许 Vernal/宿主进行拆分、改名或
/// 补充容器元数据。
///
/// 对应 Java: `com.yomahub.liteflow.spi.DeclComponentParser`。
pub trait DeclComponentParser: SpiPriority + Send + Sync {
    /// 解析一个声明式组件包装对象。
    ///
    /// 对应 Java: `DeclComponentParser#parseDeclBean(Class)`。
    fn parse_decl_bean(&self, decl_warp_bean: DeclWarpBean) -> LFResult<Vec<DeclWarpBean>>;

    /// 使用显式节点 ID 与名称解析声明式组件。
    ///
    /// 对应 Java: `DeclComponentParser#parseDeclBean(Class,String,String)`。
    fn parse_decl_bean_with_identity(
        &self,
        mut decl_warp_bean: DeclWarpBean,
        node_id: &str,
        node_name: &str,
    ) -> LFResult<Vec<DeclWarpBean>> {
        decl_warp_bean.set_node_id(node_id);
        decl_warp_bean.set_node_name(node_name);
        self.parse_decl_bean(decl_warp_bean)
    }
}
