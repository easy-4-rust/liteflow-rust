use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use liteflow_core::common::entity::ValidationResp;
use liteflow_core::el::NodeRef;
use liteflow_core::enums::{NodeTypeEnum, ScriptTypeEnum};
use liteflow_core::exception::LFResult;
use liteflow_core::lifecycle::{LifeCycleHolder, PostProcessScriptEngineInitLifeCycle};
use liteflow_core::script::{
    RhaiScriptExecutor, ScriptExecuteWrap, ScriptExecutor, ScriptExecutorComponent, ScriptKind,
};
use liteflow_core::slot::{CmpContext, Frame, Slot};
use liteflow_core::spi::{CmpAroundAspect, CmpAroundAspectHolder, SpiPriority};
use liteflow_core::{FlowBus, LiteflowError};
use serde_json::Value;

#[derive(Default)]
struct HookCounts {
    before: AtomicUsize,
    after: AtomicUsize,
    success: AtomicUsize,
    error: AtomicUsize,
}

struct TrackingAspect {
    counts: Arc<HookCounts>,
}

impl SpiPriority for TrackingAspect {
    fn priority(&self) -> i32 {
        1
    }
}

impl CmpAroundAspect for TrackingAspect {
    fn before_process(&self, _node_id: &str, _slot: &Slot) {
        self.counts.before.fetch_add(1, Ordering::SeqCst);
    }

    fn after_process(&self, _node_id: &str, _slot: &Slot) {
        self.counts.after.fetch_add(1, Ordering::SeqCst);
    }

    fn on_success(&self, _node_id: &str, _slot: &Slot) {
        self.counts.success.fetch_add(1, Ordering::SeqCst);
    }

    fn on_error(&self, _node_id: &str, _slot: &Slot, _error: &LiteflowError) {
        self.counts.error.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct TrackingLifeCycle {
    languages: Mutex<Vec<String>>,
}

impl PostProcessScriptEngineInitLifeCycle for TrackingLifeCycle {
    fn post_process_after_script_engine_init(&self, language: &str) {
        self.languages.lock().unwrap().push(language.to_string());
    }
}

impl liteflow_core::LifeCycle for TrackingLifeCycle {
    fn register_life_cycle(
        self: Arc<Self>,
        life_cycle_holder: &mut liteflow_core::LifeCycleHolder,
    ) {
        life_cycle_holder.script_engine_init.push(self);
    }
}

fn context() -> CmpContext {
    CmpContext {
        inner: Arc::new(Slot::new(
            "script-hook-request".to_string(),
            "script-hook-chain",
            Value::Null,
        )),
        node: NodeRef::new("script-hook-node"),
        frame: Frame::root().with_current_chain_id("script-hook-chain"),
    }
}

/// 验证 ScriptExecutor 初始化、访问控制与四类切面进入真实 Holder。
#[test]
fn script_executor_java_lifecycle_and_hook_entries_drive_registered_runtime_state() {
    let lifecycle = Arc::new(TrackingLifeCycle::default());
    let mut holder = LifeCycleHolder::default();
    holder
        .script_engine_init
        .push(lifecycle.clone() as Arc<dyn PostProcessScriptEngineInitLifeCycle>);

    let executor = RhaiScriptExecutor::new();
    executor.init(&holder).unwrap();
    assert_eq!(lifecycle.languages.lock().unwrap().as_slice(), ["rhai"]);

    let counts = Arc::new(HookCounts::default());
    CmpAroundAspectHolder::register(Arc::new(TrackingAspect {
        counts: Arc::clone(&counts),
    }));

    let context = context();
    let wrap = ScriptExecuteWrap::from_context(&context);
    assert!(executor.execute_is_access(&wrap, &context));
    assert!(!executor.execute_is_continue_on_error(&wrap, &context));
    assert!(!executor.execute_is_end(&wrap, &context));

    executor.execute_before_process(&wrap, &context);
    executor.execute_on_success(&wrap, &context);
    executor.execute_on_error(
        &wrap,
        &context,
        &LiteflowError::Custom("script failure".to_string()),
    );
    executor.execute_after_process(&wrap, &context);
    executor.execute_rollback(&wrap, &context).unwrap();

    assert_eq!(counts.before.load(Ordering::SeqCst), 1);
    assert_eq!(counts.success.load(Ordering::SeqCst), 1);
    assert_eq!(counts.error.load(Ordering::SeqCst), 1);
    assert_eq!(counts.after.load(Ordering::SeqCst), 1);
    CmpAroundAspectHolder::clean();
}

#[derive(Default)]
struct RuntimeHookCounts {
    before: AtomicUsize,
    error: AtomicUsize,
    after: AtomicUsize,
    continue_check: AtomicUsize,
    end_check: AtomicUsize,
}

struct ContinueAfterErrorExecutor {
    counts: Arc<RuntimeHookCounts>,
}

impl ScriptExecutor for ContinueAfterErrorExecutor {
    fn load(&self, _node_id: &str, _script: &str) -> LFResult<()> {
        Ok(())
    }

    fn unload(&self, _node_id: &str) -> LFResult<()> {
        Ok(())
    }

    fn node_ids(&self) -> LFResult<Vec<String>> {
        Ok(vec!["runtime_script".to_string()])
    }

    fn execute_script(&self, _node_id: &str, _ctx: &CmpContext) -> LFResult<Value> {
        Err(LiteflowError::Custom("runtime script failure".to_string()))
    }

    fn clean_cache(&self) -> LFResult<()> {
        Ok(())
    }

    fn script_type(&self) -> ScriptTypeEnum {
        ScriptTypeEnum::Custom
    }

    fn compile(&self, _script: &str) -> LFResult<()> {
        Ok(())
    }

    fn validate_with_ex(&self, _script: &str) -> ValidationResp {
        ValidationResp::success()
    }

    fn execute_is_continue_on_error(
        &self,
        _wrap: &ScriptExecuteWrap,
        _context: &CmpContext,
    ) -> bool {
        self.counts.continue_check.fetch_add(1, Ordering::SeqCst);
        true
    }

    fn execute_is_end(&self, _wrap: &ScriptExecuteWrap, _context: &CmpContext) -> bool {
        self.counts.end_check.fetch_add(1, Ordering::SeqCst);
        false
    }

    fn execute_before_process(&self, _wrap: &ScriptExecuteWrap, _context: &CmpContext) {
        self.counts.before.fetch_add(1, Ordering::SeqCst);
    }

    fn execute_after_process(&self, _wrap: &ScriptExecuteWrap, _context: &CmpContext) {
        self.counts.after.fetch_add(1, Ordering::SeqCst);
    }

    fn execute_on_error(
        &self,
        _wrap: &ScriptExecuteWrap,
        _context: &CmpContext,
        _error: &LiteflowError,
    ) {
        self.counts.error.fetch_add(1, Ordering::SeqCst);
    }
}

/// 验证执行器钩子不是孤立 API，而是由真实 Node 错误与 continue 主干调用。
#[tokio::test]
async fn script_executor_hooks_are_reached_by_real_node_execution() {
    let counts = Arc::new(RuntimeHookCounts::default());
    let executor = Arc::new(ContinueAfterErrorExecutor {
        counts: Arc::clone(&counts),
    });
    let component = Arc::new(ScriptExecutorComponent::new(
        "runtime_script",
        ScriptKind::Common,
        executor as Arc<dyn ScriptExecutor>,
    ));

    let bus = FlowBus::new();
    bus.add_node("runtime_script", None, NodeTypeEnum::Script, component)
        .unwrap();
    bus.add_chain("runtime_chain", "THEN(runtime_script)")
        .unwrap();

    let response = bus.execute("runtime_chain").await;
    assert!(response.is_success(), "continue-on-error 应吞掉脚本错误");
    assert_eq!(counts.before.load(Ordering::SeqCst), 1);
    assert_eq!(counts.error.load(Ordering::SeqCst), 1);
    assert_eq!(counts.after.load(Ordering::SeqCst), 1);
    assert_eq!(counts.continue_check.load(Ordering::SeqCst), 1);
    assert_eq!(counts.end_check.load(Ordering::SeqCst), 1);
}
