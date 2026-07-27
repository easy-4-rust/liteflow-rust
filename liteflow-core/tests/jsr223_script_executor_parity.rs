//! Java `JSR223ScriptExecutor` 缓存和真实脚本执行入口回归测试。

use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::script::jsr223::JSR223ScriptExecutor;
use liteflow_core::script::{ScriptExecuteWrap, ScriptKind};
use liteflow_core::{CmpContext, FlowBus, LiteflowError, NodeComponent};
use serde_json::{Value, json};

struct JavaNamedScriptComponent {
    executor: Arc<JSR223ScriptExecutor>,
}

#[async_trait]
impl NodeComponent for JavaNamedScriptComponent {
    async fn process(&self, context: &CmpContext) -> Result<Value, LiteflowError> {
        self.executor
            .execute_script(&ScriptExecuteWrap::from_context(context), context)
            .await
    }
}

#[tokio::test]
async fn java_named_cache_entries_drive_real_rhai_execution() {
    let executor = Arc::new(JSR223ScriptExecutor::new("rhai", ScriptKind::Common));
    executor.init().expect("Rhai 执行器应可初始化");
    executor
        .load("javaNamedScript", r#"data["answer"] = 42; node_id"#)
        .expect("脚本应可编译");
    assert_eq!(executor.get_node_ids(), vec!["javaNamedScript"]);

    let bus = FlowBus::new();
    bus.register(
        "javaNamedScript",
        JavaNamedScriptComponent {
            executor: Arc::clone(&executor),
        },
    );
    bus.add_chain("javaNamedScriptChain", "THEN(javaNamedScript)")
        .expect("链路应可构建");

    let response = bus.execute("javaNamedScriptChain").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("answer"), Some(json!(42)));

    executor.un_load("javaNamedScript");
    assert!(executor.get_node_ids().is_empty());
    executor.load("temporary", "1").expect("清理前脚本应可加载");
    executor.clean_cache();
    assert!(executor.get_node_ids().is_empty());
}
