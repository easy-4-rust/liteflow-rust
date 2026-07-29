use crate::builder::prop::NodePropBean;
use crate::core::{ComponentInitializer, NodeComponent};
use crate::enums::NodeTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::script::ScriptKind;
use crate::spi::PathContentParserHolder;
use std::sync::Arc;

/// Rust 原生节点构建器。
///
/// Java 版通过 class name 反射实例化普通组件；Rust 无运行期 classpath，
/// 因此普通组件通过 `set_component`/`set_component_arc` 提供实例，脚本组件
/// 仍按 id/type/script/language 构建并注册到 FlowBus。
/// 对应 Java: `com.yomahub.liteflow.builder.LiteFlowNodeBuilder`。
pub struct LiteFlowNodeBuilder {
    bus: FlowBus,
    node_id: Option<String>,
    name: Option<String>,
    clazz: Option<String>,
    node_type: Option<NodeTypeEnum>,
    script: Option<String>,
    file: Option<String>,
    language: Option<String>,
    component: Option<Arc<dyn NodeComponent>>,
}

impl LiteFlowNodeBuilder {
    /// 创建未指定类型的节点构建器。对应 Java: `createNode`。
    pub fn create_node(bus: &FlowBus) -> Self {
        Self::new(bus, None)
    }

    /// 创建普通节点构建器。对应 Java: `createCommonNode`。
    pub fn create_common_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::Common))
    }

    /// 创建选择节点构建器。对应 Java: `createSwitchNode`。
    pub fn create_switch_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::Switch))
    }

    /// 创建布尔节点构建器。对应 Java: `createBooleanNode`。
    pub fn create_boolean_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::Boolean))
    }

    /// 创建循环次数节点构建器。对应 Java: `createForNode`。
    pub fn create_for_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::For))
    }

    /// 创建迭代节点构建器。对应 Java: `createIteratorNode`。
    pub fn create_iterator_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::Iterator))
    }

    /// 创建普通脚本节点构建器。对应 Java: `createScriptNode`。
    pub fn create_script_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::Script))
    }

    /// 创建选择脚本节点构建器。对应 Java: `createScriptSwitchNode`。
    pub fn create_script_switch_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::SwitchScript))
    }

    /// 创建布尔脚本节点构建器。对应 Java: `createScriptBooleanNode`。
    pub fn create_script_boolean_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::BooleanScript))
    }

    /// 创建循环次数脚本节点构建器。对应 Java: `createScriptForNode`。
    pub fn create_script_for_node(bus: &FlowBus) -> Self {
        Self::new(bus, Some(NodeTypeEnum::ForScript))
    }

    fn new(bus: &FlowBus, node_type: Option<NodeTypeEnum>) -> Self {
        Self {
            bus: bus.clone(),
            node_id: None,
            name: None,
            clazz: None,
            node_type,
            script: None,
            file: None,
            language: None,
            component: None,
        }
    }

    /// 从 NodePropBean 创建 builder。
    ///
    /// Java ParserHelper 会反射 clazz 推断类型；Rust 端要求配置显式 type，
    /// 或该 id 已由应用注册，此时按 Common 处理。
    pub fn from_prop(bus: &FlowBus, node_prop_bean: NodePropBean) -> LFResult<Self> {
        let mut builder = Self::create_node(bus);
        if let Some(id) = node_prop_bean.id {
            builder = builder.set_id(id);
        }
        if let Some(name) = node_prop_bean.name {
            builder = builder.set_name(name);
        }
        if let Some(clazz) = node_prop_bean.clazz {
            builder = builder.set_clazz(clazz);
        }
        if let Some(script) = node_prop_bean.script {
            builder = builder.set_script(script);
        }
        if let Some(file) = node_prop_bean.file {
            builder = builder.set_file(file);
        }
        if let Some(language) = node_prop_bean.language {
            builder = builder.set_language(language);
        }
        if let Some(node_type) = node_prop_bean.node_type {
            let parsed = NodeTypeEnum::get_enum_by_code(node_type.trim()).ok_or_else(|| {
                LiteflowError::NodeTypeNotSupport(format!(
                    "type [{}] is not support",
                    node_type.trim()
                ))
            })?;
            builder = builder.set_type(parsed);
        } else if builder
            .node_id
            .as_deref()
            .is_some_and(|id| bus.contains_node(id))
        {
            builder = builder.set_type(NodeTypeEnum::Common);
        }
        Ok(builder)
    }

    /// 设置节点 id。空白值不覆盖已有配置。对应 Java: `setId`。
    pub fn set_id(mut self, node_id: impl Into<String>) -> Self {
        let node_id = node_id.into();
        if !node_id.trim().is_empty() {
            self.node_id = Some(node_id.trim().to_string());
        }
        self
    }

    /// 设置节点名称。空白值不覆盖已有配置。对应 Java: `setName`。
    pub fn set_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if !name.trim().is_empty() {
            self.name = Some(name.trim().to_string());
        }
        self
    }

    /// 保留 Java 类名诊断元数据。对应 Java: `setClazz(String)`。
    pub fn set_clazz(mut self, clazz: impl Into<String>) -> Self {
        let clazz = clazz.into();
        if !clazz.trim().is_empty() {
            self.clazz = Some(clazz.trim().to_string());
        }
        self
    }

    /// 设置节点类型。对应 Java: `setType`。
    pub fn set_type(mut self, node_type: NodeTypeEnum) -> Self {
        self.node_type = Some(node_type);
        self
    }

    /// 设置脚本文本。对应 Java: `setScript`。
    pub fn set_script(mut self, script: impl Into<String>) -> Self {
        self.script = Some(script.into());
        self
    }

    /// 设置脚本文件路径；build 时由 PathContentParser SPI 读取。
    /// 对应 Java: `setFile`。
    pub fn set_file(mut self, file: impl Into<String>) -> Self {
        let file = file.into();
        if !file.trim().is_empty() {
            self.file = Some(file.trim().to_string());
        }
        self
    }

    /// 设置脚本语言。空白值不覆盖已有配置。对应 Java: `setLanguage`。
    pub fn set_language(mut self, language: impl Into<String>) -> Self {
        let language = language.into();
        if !language.trim().is_empty() {
            self.language = Some(language.trim().to_string());
        }
        self
    }

    /// 设置普通节点组件实例（Rust 替代 Java class 反射）。
    pub fn set_component<C: NodeComponent>(mut self, component: C) -> Self {
        self.component = Some(Arc::new(component));
        self
    }

    /// 设置对象安全的普通节点组件实例。
    pub fn set_component_arc(mut self, component: Arc<dyn NodeComponent>) -> Self {
        self.component = Some(component);
        self
    }

    /// 校验并注册节点。对应 Java: `LiteFlowNodeBuilder#build`。
    pub fn build(self) -> LFResult<()> {
        // Java checkBuild 会一次收集全部前置错误，并按固定顺序输出。
        let mut build_errors = Vec::new();
        if self.node_id.is_none() {
            build_errors.push("id is blank");
        }
        if self.node_type.is_none() {
            build_errors.push("type is null");
        }
        if !build_errors.is_empty() {
            return Err(LiteflowError::NodeBuild(format!(
                "[{}]",
                build_errors.join(",")
            )));
        }
        let node_id = self.node_id.clone().expect("前置校验已保证节点 id 存在");
        let node_type = self.node_type.expect("前置校验已保证节点类型存在");

        if node_type.is_script() {
            return self.build_script_node(node_id, node_type);
        }
        if node_type == NodeTypeEnum::Fallback {
            return Err(LiteflowError::NodeTypeNotSupport(
                "fallback nodes require liteflow-derive fallback registration".to_string(),
            ));
        }

        let Some(component) = self.component else {
            if self.bus.contains_node(&node_id) {
                return Ok(());
            }
            let clazz = self.clazz.unwrap_or_default();
            return Err(LiteflowError::NodeBuild(format!(
                "node[{node_id}] component is missing; Rust cannot reflect class[{clazz}], use set_component"
            )));
        };
        let component = ComponentInitializer::load_instance().init_component(
            component,
            node_type,
            self.name.as_deref(),
            &node_id,
        )?;
        self.bus.insert_initialized_arc(node_id, component)
    }

    fn build_script_node(self, node_id: String, node_type: NodeTypeEnum) -> LFResult<()> {
        let script = if let Some(file) = self.file {
            let parser = PathContentParserHolder::load_context_aware();
            parser
                .parse_content(&[file])?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    LiteflowError::NodeBuild(format!(
                        "An exception occurred while building the node[{node_id}], script file is empty"
                    ))
                })?
        } else {
            self.script.unwrap_or_default()
        };
        if script.trim().is_empty() {
            return Err(LiteflowError::NodeBuild(format!(
                "An exception occurred while building the node[{node_id}], script is blank"
            )));
        }
        let kind = match node_type {
            NodeTypeEnum::Script => ScriptKind::Common,
            NodeTypeEnum::SwitchScript => ScriptKind::Switch,
            NodeTypeEnum::BooleanScript
            | NodeTypeEnum::IfScript
            | NodeTypeEnum::WhileScript
            | NodeTypeEnum::BreakScript => ScriptKind::Boolean,
            NodeTypeEnum::ForScript => ScriptKind::For,
            _ => {
                return Err(LiteflowError::NodeTypeNotSupport(format!(
                    "type [{}] is not a script node",
                    node_type.get_code()
                )));
            }
        };
        let language = self.language.as_deref().unwrap_or("rhai");
        self.bus
            .register_script_typed(&node_id, language, kind, &script)?;
        if self.name.is_some() {
            let component = self
                .bus
                .get_node(&node_id)
                .expect("script component was just registered");
            let component = ComponentInitializer::load_instance().init_component(
                component,
                node_type,
                self.name.as_deref(),
                &node_id,
            )?;
            self.bus.insert_initialized_arc(node_id, component)?;
        }
        Ok(())
    }
}
