use liteflow_core::{
    CmpContext, LiteflowError, NodeComponent, NodeTypeEnum, async_trait, serde_json::Value,
};

struct ExplicitBooleanComponent;

#[async_trait]
impl NodeComponent for ExplicitBooleanComponent {
    async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Bool(true))
    }

    fn node_type(&self) -> Option<NodeTypeEnum> {
        Some(NodeTypeEnum::Boolean)
    }
}

/// 验证 Java 可变枚举元数据在 Rust 中执行真实、可校验的类型转换。
#[test]
fn node_type_enum_mutators_preserve_type_invariants() {
    let mut node_type = NodeTypeEnum::Common;
    assert!(node_type.set_code("switch"));
    assert_eq!(node_type, NodeTypeEnum::Switch);
    assert!(!node_type.set_code("unknown"));
    assert_eq!(node_type, NodeTypeEnum::Switch);

    assert!(node_type.set_name("循环次数"));
    assert_eq!(node_type, NodeTypeEnum::For);
    assert!(node_type.set_script(true));
    assert_eq!(node_type, NodeTypeEnum::ForScript);
    assert_eq!(node_type.get_mapping_clazz(), Some("ScriptForComponent"));

    assert!(node_type.set_mapping_clazz(Some("com.yomahub.liteflow.core.ScriptSwitchComponent")));
    assert_eq!(node_type, NodeTypeEnum::SwitchScript);
    assert!(node_type.set_mapping_clazz(None));
    assert_eq!(node_type, NodeTypeEnum::Fallback);
    assert_eq!(node_type.get_mapping_clazz(), None);
}

/// 验证父类名称与组件注册元数据两条 Java 推断路径的 Rust 对等实现。
#[test]
fn node_type_enum_guesses_from_mapping_name_and_component_metadata() {
    assert_eq!(
        NodeTypeEnum::guess_type_by_super_clazz("com.yomahub.liteflow.core.NodeIteratorComponent"),
        Some(NodeTypeEnum::Iterator)
    );
    assert_eq!(
        NodeTypeEnum::guess_type_by_super_clazz("demo.UnknownComponent"),
        None
    );
    assert_eq!(
        NodeTypeEnum::guess_type(&ExplicitBooleanComponent),
        Some(NodeTypeEnum::Boolean)
    );
}
