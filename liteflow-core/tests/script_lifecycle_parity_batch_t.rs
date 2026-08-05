//! 脚本节点卸载/重载/缓存生命周期补测（批次 T）。
//!
//! 覆盖：
//! - `FlowBus#unloadScriptNode` 的缺失节点/普通节点/脚本节点三路径
//! - `FlowBus#reloadScript` 的缺失节点静默返回与真实重载
//! - `FlowBus#cleanScriptCache` 只清缓存、保留节点元数据
//! - `ScriptExecutorFactory#register` 自定义语言与空白语言拒绝

use liteflow_core::script::{ScriptExecutorFactory, ScriptKind};
use liteflow_core::{FlowBus, NodeComponent, cmp};
use serde_json::{Value, json};
use std::sync::Arc;

/// unloadScriptNode 的缺失/普通/脚本节点路径。
#[tokio::test]
async fn unload_script_node_paths() {
    let bus = FlowBus::new();
    // 缺失节点返回 false
    assert!(!bus.unload_script_node("missing").unwrap());

    // 普通节点返回 false
    bus.register("plain", cmp(|_| async { Ok(Value::Null) }));
    assert!(!bus.unload_script_node("plain").unwrap());

    // 脚本节点卸载返回 true 并删除元数据
    bus.register_script("script_u", "rhai", "let x = 1;")
        .unwrap();
    assert!(bus.unload_script_node("script_u").unwrap());
    assert!(!bus.contains_node("script_u"));
    // 再次卸载返回 false
    assert!(!bus.unload_script_node("script_u").unwrap());
}

/// reloadScript 的缺失节点静默返回与真实重载。
#[tokio::test]
async fn reload_script_paths() {
    let bus = FlowBus::new();
    // 缺失节点静默返回 Ok（Java 语义）
    assert!(bus.reload_script("missing", "let y = 1;").is_ok());

    // 普通节点静默返回 Ok
    bus.register("plain_r", cmp(|_| async { Ok(Value::Null) }));
    assert!(bus.reload_script("plain_r", "let y = 1;").is_ok());

    // 脚本节点重载成功
    bus.register_script("script_r", "rhai", "let a = 1;")
        .unwrap();
    assert!(bus.reload_script("script_r", "let b = 2;").is_ok());
    assert!(bus.contains_node("script_r"));
}

/// cleanScriptCache 只清编译缓存、保留节点元数据。
#[tokio::test]
async fn clean_script_cache_preserves_nodes() {
    let bus = FlowBus::new();
    bus.register_script("script_c", "rhai", "let c = 1;")
        .unwrap();
    assert!(bus.clean_script_cache().is_ok());
    // 节点仍保留
    assert!(bus.contains_node("script_c"));
    // 缓存清理后可重载
    assert!(bus.reload_script("script_c", "let c = 2;").is_ok());
}

/// ScriptExecutorFactory 自定义语言注册与空白语言拒绝。
#[test]
fn executor_factory_register_languages() {
    // 空白语言拒绝
    let error = ScriptExecutorFactory::register("", |_id, _kind, _script| {
        Err(liteflow_core::LiteflowError::Script {
            node: String::new(),
            msg: "unused".to_string(),
        })
    })
    .expect_err("空白语言应拒绝");
    assert!(error.to_string().contains("blank"));

    // 自定义语言注册后可通过 build 调用（ScriptComponentBuilder 是 fn 指针）
    fn custom_builder(
        _node_id: &str,
        _kind: ScriptKind,
        _script: &str,
    ) -> liteflow_core::exception::LFResult<Arc<dyn NodeComponent>> {
        Ok(Arc::new(cmp(|_| async { Ok(json!("custom")) })))
    }
    ScriptExecutorFactory::register("custom_lang", custom_builder).expect("注册自定义语言");
    assert!(ScriptExecutorFactory::build("custom_lang", "n1", ScriptKind::Common, "s").is_ok());
    // 未注册语言失败
    assert!(ScriptExecutorFactory::build("not_registered", "n1", ScriptKind::Common, "s").is_err());
}
