//! Java `LiteFlowProxyUtil` 的 Rust 映射。

use std::sync::Arc;

use crate::core::DeclComponent;
use crate::exception::LFResult;
use crate::flow::FlowBus;

use super::{DeclComponentProxy, DeclWarpBean};

/// 声明式组件代理通用工具。
///
/// 对应 Java: `com.yomahub.liteflow.core.proxy.LiteFlowProxyUtil`。
pub struct LiteFlowProxyUtil;

impl LiteFlowProxyUtil {
    /// 判断包装对象是否包含 `@LiteflowMethod` 元数据。
    ///
    /// Rust 的过程宏已在编译期完成 Java `Class#getMethods` 扫描。对应 Java:
    /// `LiteFlowProxyUtil#isDeclareCmp`。
    #[must_use]
    pub fn is_declare_cmp(decl_warp_bean: &DeclWarpBean) -> bool {
        !decl_warp_bean.method_wrap_bean_list().is_empty()
    }

    /// 将声明式组件包装对象转换为可注册的组件代理。
    ///
    /// 参数 `decl_warp_bean` 保存过程宏生成的类型、方法、重试和参数事实元数据；
    /// 返回值通过 `DeclComponent` 静态分派表承担 Java `NodeComponent` 动态子类的
    /// 职责。对应 Java: `LiteFlowProxyUtil#proxy2NodeComponent`。
    pub fn proxy2_node_component(decl_warp_bean: DeclWarpBean) -> LFResult<Arc<dyn DeclComponent>> {
        DeclComponentProxy::new(decl_warp_bean).get_proxy()
    }

    /// 生成声明式组件代理。
    ///
    /// 这是早期 Rust API 的描述性名称；实现统一委托 Java 对等入口
    /// `proxy2_node_component`。
    pub fn proxy_to_decl_component(
        decl_warp_bean: DeclWarpBean,
    ) -> LFResult<Arc<dyn DeclComponent>> {
        Self::proxy2_node_component(decl_warp_bean)
    }

    /// 生成代理并注册到 FlowBus。
    ///
    /// 这是 Java `ContextAware#registerDeclWrapBean` 与
    /// `FlowBus#getNodeComponentList` 的 Rust 显式装配入口。
    pub fn register_decl_warp(flow_bus: &FlowBus, decl_warp_bean: DeclWarpBean) -> LFResult<()> {
        let node_id = decl_warp_bean.node_id().to_string();
        let proxy = Self::proxy2_node_component(decl_warp_bean)?;
        flow_bus.register_decl(node_id, proxy);
        Ok(())
    }

    /// 判断类型名是否为 CGLIB/ByteBuddy 风格代理名。
    ///
    /// 对应 Java: `LiteFlowProxyUtil#isCglibProxyClass`。
    #[must_use]
    pub fn is_cglib_proxy_class(class_name: &str) -> bool {
        class_name.contains("$$")
    }

    /// 从代理类型名中返回用户类型名。
    ///
    /// Rust 不产生 CGLIB 继承类；该方法保留 Java 配置/诊断字符串兼容语义。对应
    /// Java: `LiteFlowProxyUtil#getUserClass`。
    #[must_use]
    pub fn get_user_class(class_name: &str) -> &str {
        class_name
            .split_once("$$")
            .map_or(class_name, |(user_class, _)| user_class)
    }
}
