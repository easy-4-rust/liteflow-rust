//! 运行时核心对象的 Java 命名入口与真实行为测试。

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::core::NodeComponent;
use liteflow_core::flow::NodeInstanceIdManageSpi;
use liteflow_core::meta::LiteflowMetaOperator;
use liteflow_core::script::{ScriptExecutorFactory, ScriptKind};
use liteflow_core::{
    CmpContext, ContextBeanFactory, DataBus, ExecuteOption, FlowBus, FlowExecutor, LFResult,
    LiteflowError, cmp,
};
use serde_json::Value;

#[derive(Default)]
struct RuntimeContext {
    value: usize,
}

struct FixedInstanceIdSpi {
    lines: Vec<String>,
}

impl NodeInstanceIdManageSpi for FixedInstanceIdSpi {
    fn gen_instance_id(&self, _chain_id: &str, node_id: &str, occurrence: usize) -> String {
        format!("{node_id}-fixed-{occurrence}")
    }

    fn read_instance_id_file(&self, _chain_id: &str) -> LFResult<Vec<String>> {
        Ok(self.lines.clone())
    }
}

struct FactoryComponent;

#[async_trait]
impl NodeComponent for FactoryComponent {
    async fn process(&self, _context: &CmpContext) -> LFResult<Value> {
        Ok(Value::String("factory".to_string()))
    }

    fn name(&self) -> &str {
        "factory-component"
    }
}

fn build_factory_component(
    _node_id: &str,
    _kind: ScriptKind,
    script: &str,
) -> LFResult<Arc<dyn NodeComponent>> {
    if script != "real-script" {
        return Err(LiteflowError::Script {
            node: "factory-node".to_string(),
            msg: "unexpected script".to_string(),
        });
    }
    Ok(Arc::new(FactoryComponent))
}

#[tokio::test]
async fn execute_option_context_class_constructs_a_fresh_real_context_at_execution_time() {
    let bus = FlowBus::new();
    bus.register(
        "read",
        cmp(|context| async move {
            let runtime_context = context
                .bean::<RuntimeContext>("runtimeContext")
                .expect("context_class 应按 Java 默认类名规则登记 Bean");
            context.set_data("context_value", Value::from(runtime_context.value as u64));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("context-class-chain", "THEN(read)").unwrap();
    let executor = FlowExecutor::new(bus);
    let option = ExecuteOption::of()
        .request_id("context-class-request")
        .context_class::<RuntimeContext>();

    assert_eq!(option.get_context_bean_classes().len(), 1);
    let response = executor
        .execute2_resp("context-class-chain", Value::Null, Some(option))
        .await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("context_value"), Some(Value::from(0)));
    assert!(
        response
            .get_context_bean::<RuntimeContext>("runtimeContext")
            .is_some()
    );
}

#[test]
fn data_bus_class_and_bean_overloads_create_queryable_slots() {
    let option = ExecuteOption::of().context_class::<RuntimeContext>();
    let context_bean_classes: Vec<ContextBeanFactory> = option.get_context_bean_classes().to_vec();
    let class_slot_index = DataBus::offer_slot_by_class(&context_bean_classes);
    let class_slot = DataBus::get_slot(class_slot_index).expect("类型工厂应创建 Slot");
    assert!(
        class_slot
            .get_context_bean::<RuntimeContext>("runtimeContext")
            .is_some()
    );
    assert!(DataBus::release_slot(class_slot_index));

    let bean: Arc<dyn Any + Send + Sync> = Arc::new(RuntimeContext { value: 7 });
    let bean_slot_index = DataBus::offer_slot_by_bean(vec![("customContext".to_string(), bean)]);
    let bean_slot = DataBus::get_slot(bean_slot_index).expect("Bean 实例应创建 Slot");
    assert_eq!(
        bean_slot
            .get_context_bean::<RuntimeContext>("customContext")
            .expect("具名 Bean 应可查询")
            .value,
        7
    );
    assert!(DataBus::release_slot(bean_slot_index));
}

#[test]
fn metadata_instance_id_methods_use_the_configured_runtime_spi() {
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.add_chain("instance-chain", r#"THEN(a.tag("first"), a.tag("second"))"#)
        .unwrap();
    bus.set_instance_id_spi(Arc::new(FixedInstanceIdSpi {
        lines: vec![
            "md5".to_string(),
            r#"[{"chainId":"instance-chain","nodeId":"a","instanceId":"a-fixed-0","index":0},{"chainId":"instance-chain","nodeId":"a","instanceId":"a-fixed-1","index":1}]"#
                .to_string(),
        ],
    }));
    let metadata = LiteflowMetaOperator::new(bus);

    assert_eq!(
        metadata
            .get_node_instance_ids("instance-chain", "a")
            .unwrap(),
        vec!["a-fixed-0".to_string(), "a-fixed-1".to_string()]
    );
    assert_eq!(
        metadata
            .get_node_index("instance-chain", "a-fixed-1")
            .unwrap(),
        1
    );
    assert_eq!(
        metadata
            .get_node("instance-chain", "a-fixed-1")
            .unwrap()
            .expect("实例 ID 应定位到第二个节点")
            .tag
            .as_deref(),
        Some("second")
    );
    assert!(
        metadata
            .get_node("instance-chain", "missing")
            .unwrap()
            .is_none()
    );
}

#[test]
fn script_executor_factory_java_named_methods_resolve_and_clean_real_builders() {
    ScriptExecutorFactory::register("runtime-named-api", build_factory_component).unwrap();
    let factory = ScriptExecutorFactory::load_instance();
    let builder = factory.get_script_executor("runtime-named-api").unwrap();
    let component = builder("factory-node", ScriptKind::Common, "real-script").unwrap();

    assert_eq!(component.name(), "factory-component");
    factory.clean_script_cache();
    assert!(factory.get_script_executor("runtime-named-api").is_err());
}
