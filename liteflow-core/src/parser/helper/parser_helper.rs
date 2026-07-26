use crate::builder::{LiteFlowNodeBuilder, NodePropBean};
use crate::enums::NodeTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::script::ScriptKind;

/// 规则解析器共享的节点校验与构建助手。
///
/// Java 对象还集中承载 XML/JSON 链路遍历；Rust 版的格式遍历分别由
/// `BaseXmlFlowParser`、`BaseJsonFlowParser` 完成，而本对象保留两条路径共同
/// 依赖的节点类型推断、合法性校验和真实注册职责。
///
/// 对应 Java: `com.yomahub.liteflow.parser.helper.ParserHelper`。
pub struct ParserHelper;

impl ParserHelper {
    /// 校验节点中间属性并将节点真实注册到流程总线。
    ///
    /// 参数 `bus` 是目标流程总线，`node_prop_bean` 对应 Java 构建中间对象；
    /// 成功返回空值，失败返回具体的类缺失、类型不可推断、类型不支持或构建错误。
    /// 对应 Java: `ParserHelper#buildNode`。
    pub fn build_node(bus: &FlowBus, mut node_prop_bean: NodePropBean) -> LFResult<()> {
        let id = node_prop_bean.id.clone().unwrap_or_default();

        // Rust 不支持 Java Class.forName；应用预注册的同 id 组件是 class 节点的
        // 可执行映射。未注册时明确报告边界，不能把类名伪装成已迁移组件。
        if node_prop_bean
            .clazz
            .as_deref()
            .is_some_and(|clazz| !clazz.trim().is_empty())
        {
            if bus.contains_node(&id) {
                node_prop_bean.node_type = Some(NodeTypeEnum::Common.get_code().to_string());
            } else {
                let clazz = node_prop_bean.clazz.as_deref().unwrap_or_default();
                return Err(LiteflowError::NodeClassNotFound(format!(
                    "cannot find the node[{clazz}]"
                )));
            }
        }

        // iterator_script 是早期 Rust 规则格式；保留其真实执行语义，但不将其
        // 混入 Java NodeTypeEnum 对照枚举。
        if node_prop_bean.node_type.as_deref() == Some("iterator_script") {
            let script = node_prop_bean
                .script
                .as_deref()
                .ok_or_else(|| LiteflowError::NodeBuild(format!("node[{id}] missing script")))?;
            return bus.register_script_typed(
                id,
                node_prop_bean.language.as_deref().unwrap_or("rhai"),
                ScriptKind::Iterator,
                script,
            );
        }

        let node_type = node_prop_bean
            .node_type
            .as_deref()
            .filter(|node_type| !node_type.trim().is_empty())
            .ok_or_else(|| {
                LiteflowError::NodeTypeCanNotGuess(format!(
                    "cannot guess the type of node[{}]",
                    node_prop_bean.clazz.as_deref().unwrap_or_default()
                ))
            })?;
        if NodeTypeEnum::get_enum_by_code(node_type.trim()).is_none() {
            return Err(LiteflowError::NodeTypeNotSupport(format!(
                "type [{}] is not support",
                node_type.trim()
            )));
        }

        LiteFlowNodeBuilder::from_prop(bus, node_prop_bean)?.build()
    }
}
