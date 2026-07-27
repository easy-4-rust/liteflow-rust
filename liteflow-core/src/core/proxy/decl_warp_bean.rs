//! Java `DeclWarpBean` 的 Rust 映射。

use std::sync::Arc;

use crate::core::DeclComponent;
use crate::enums::NodeTypeEnum;

use super::MethodWrapBean;

/// 声明式组件 BeanDefinition 的包装对象。
///
/// Java 保存原始 Bean、Class 与反射方法；Rust 保存 `Arc<dyn DeclComponent>`、
/// `type_name` 和编译期生成的方法元数据，所有权安全地承担相同职责。
///
/// 对应 Java: `com.yomahub.liteflow.core.proxy.DeclWarpBean`。
#[derive(Clone)]
pub struct DeclWarpBean {
    node_id: String,
    node_name: String,
    node_type: NodeTypeEnum,
    raw_bean: Arc<dyn DeclComponent>,
    raw_clazz: String,
    method_wrap_bean_list: Vec<MethodWrapBean>,
}

impl DeclWarpBean {
    /// 创建声明式组件包装对象。
    #[must_use]
    pub fn new(
        node_id: impl Into<String>,
        node_name: impl Into<String>,
        node_type: NodeTypeEnum,
        raw_bean: Arc<dyn DeclComponent>,
        raw_clazz: impl Into<String>,
        method_wrap_bean_list: Vec<MethodWrapBean>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            node_name: node_name.into(),
            node_type,
            raw_bean,
            raw_clazz: raw_clazz.into(),
            method_wrap_bean_list,
        }
    }

    /// 返回节点 ID。对应 Java: `DeclWarpBean#getNodeId`。
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 返回节点 ID。
    ///
    /// 返回值对应声明式组件的 `nodeId`。对应 Java:
    /// `DeclWarpBean#getNodeId`。
    #[must_use]
    pub fn get_node_id(&self) -> &str {
        self.node_id()
    }

    /// 修改节点 ID。对应 Java: `DeclWarpBean#setNodeId`。
    pub fn set_node_id(&mut self, node_id: impl Into<String>) {
        self.node_id = node_id.into();
    }

    /// 返回节点名称。对应 Java: `DeclWarpBean#getNodeName`。
    #[must_use]
    pub fn node_name(&self) -> &str {
        &self.node_name
    }

    /// 返回节点显示名称。对应 Java: `DeclWarpBean#getNodeName`。
    #[must_use]
    pub fn get_node_name(&self) -> &str {
        self.node_name()
    }

    /// 修改节点名称。对应 Java: `DeclWarpBean#setNodeName`。
    pub fn set_node_name(&mut self, node_name: impl Into<String>) {
        self.node_name = node_name.into();
    }

    /// 返回节点类型。对应 Java: `DeclWarpBean#getNodeType`。
    #[must_use]
    pub fn node_type(&self) -> NodeTypeEnum {
        self.node_type
    }

    /// 返回声明式组件节点类型。对应 Java: `DeclWarpBean#getNodeType`。
    #[must_use]
    pub fn get_node_type(&self) -> NodeTypeEnum {
        self.node_type()
    }

    /// 修改节点类型。对应 Java: `DeclWarpBean#setNodeType`。
    pub fn set_node_type(&mut self, node_type: NodeTypeEnum) {
        self.node_type = node_type;
    }

    /// 返回原始声明式组件。对应 Java: `DeclWarpBean#getRawBean`。
    #[must_use]
    pub fn raw_bean(&self) -> &Arc<dyn DeclComponent> {
        &self.raw_bean
    }

    /// 返回原始声明式组件对象。
    ///
    /// Rust 使用线程安全 `Arc` 保存 Java `Object` 的等价对象。对应 Java:
    /// `DeclWarpBean#getRawBean`。
    #[must_use]
    pub fn get_raw_bean(&self) -> &Arc<dyn DeclComponent> {
        self.raw_bean()
    }

    /// 修改原始声明式组件。对应 Java: `DeclWarpBean#setRawBean`。
    pub fn set_raw_bean(&mut self, raw_bean: Arc<dyn DeclComponent>) {
        self.raw_bean = raw_bean;
    }

    /// 返回原始 Rust 类型名。对应 Java: `DeclWarpBean#getRawClazz`。
    #[must_use]
    pub fn raw_clazz(&self) -> &str {
        &self.raw_clazz
    }

    /// 返回原始声明式组件类型名。
    ///
    /// Rust 类型名承担 Java `Class<?>` 的诊断和注册标识职责。对应 Java:
    /// `DeclWarpBean#getRawClazz`。
    #[must_use]
    pub fn get_raw_clazz(&self) -> &str {
        self.raw_clazz()
    }

    /// 修改原始 Rust 类型名。对应 Java: `DeclWarpBean#setRawClazz`。
    pub fn set_raw_clazz(&mut self, raw_clazz: impl Into<String>) {
        self.raw_clazz = raw_clazz.into();
    }

    /// 返回声明式方法列表。对应 Java: `DeclWarpBean#getMethodWrapBeanList`。
    #[must_use]
    pub fn method_wrap_bean_list(&self) -> &[MethodWrapBean] {
        &self.method_wrap_bean_list
    }

    /// 返回声明式方法包装列表。
    ///
    /// 返回切片只读借用，语义对应 Java List getter。对应 Java:
    /// `DeclWarpBean#getMethodWrapBeanList`。
    #[must_use]
    pub fn get_method_wrap_bean_list(&self) -> &[MethodWrapBean] {
        self.method_wrap_bean_list()
    }

    /// 修改声明式方法列表。对应 Java: `DeclWarpBean#setMethodWrapBeanList`。
    pub fn set_method_wrap_bean_list(&mut self, method_wrap_bean_list: Vec<MethodWrapBean>) {
        self.method_wrap_bean_list = method_wrap_bean_list;
    }
}
