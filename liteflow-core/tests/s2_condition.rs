//! S2-D 挂接测试：condition 包补齐（ConditionKey 迁移 + 逐类语义比对补缺）。
//!
//! 覆盖：
//! - ConditionKey：15 键字符串值与 Java 常量一致、相等/哈希/反查语义
//! - 各 Condition 的 condition_type()（对应 Java getConditionType()）
//! - IF/SWITCH/FOR/WHILE/ITERATOR 的 isAccess=false 整体跳过（Java executeCondition 前置判断）
//! - WHEN 过滤 pre/finally 与 isAccess=false 分支（Java executeAsyncCondition stream 过滤）
//! - THEN add_executable 按类型分流 PRE/FINALLY（Java ThenCondition#addExecutable）
//! - CATCH 的 DO 成功后清除 slot 异常（Java CatchCondition#executeCondition removeException）

use liteflow_core::el::NodeRef;
use liteflow_core::exception::{LFResult, LiteflowError};
use liteflow_core::flow::element::ConditionKey;
use liteflow_core::flow::element::condition::BooleanConditionTypeEnum;
use liteflow_core::flow::element::condition::and_or_condition::AndOrCondition;
use liteflow_core::flow::element::condition::catch_condition::CatchCondition;
use liteflow_core::flow::element::condition::finally_condition::FinallyCondition;
use liteflow_core::flow::element::condition::for_condition::ForCondition;
use liteflow_core::flow::element::condition::if_condition::IfCondition;
use liteflow_core::flow::element::condition::iterator_condition::IteratorCondition;
use liteflow_core::flow::element::condition::pre_condition::PreCondition;
use liteflow_core::flow::element::condition::switch_condition::SwitchCondition;
use liteflow_core::flow::element::condition::then_condition::ThenCondition;
use liteflow_core::flow::element::condition::when_condition::WhenCondition;
use liteflow_core::flow::element::condition::while_condition::WhileCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::flow::element::node::Node;
use liteflow_core::slot::{CmpContext, Ctx, Frame, Slot};
use liteflow_core::{ConditionTypeEnum, FlowBus, NodeComponent, ParallelStrategyEnum, cmp};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// 记录执行轨迹的桩 Executable
struct Stub {
    sid: &'static str,
    access: bool,
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl Stub {
    fn new(
        sid: &'static str,
        access: bool,
        log: &Arc<Mutex<Vec<&'static str>>>,
    ) -> Arc<dyn Executable> {
        Arc::new(Self {
            sid,
            access,
            log: log.clone(),
        })
    }
}

#[async_trait::async_trait]
impl Executable for Stub {
    async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
        self.log.lock().unwrap().push(self.sid);
        Ok(Value::Null)
    }
    fn id(&self) -> &str {
        self.sid
    }
    async fn is_access(&self, _ctx: &Ctx, _frame: &Frame) -> bool {
        self.access
    }
}

/// 第一次可访问、后续不可访问的布尔组件，用于检测重复 `isAccess`。
struct OneShotAccessComponent {
    access_calls: Arc<AtomicUsize>,
    process_calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl NodeComponent for OneShotAccessComponent {
    async fn process(&self, _ctx: &CmpContext) -> LFResult<Value> {
        self.process_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Bool(true))
    }

    fn is_access(&self, _ctx: &CmpContext) -> bool {
        self.access_calls.fetch_add(1, Ordering::SeqCst) == 0
    }
}

fn ctx_frame() -> (Ctx, Frame) {
    let slot = Arc::new(Slot::new("req-1".to_string(), "chain1", Value::Null));
    (Ctx::new(slot), Frame::root())
}

/// ConditionKey：字符串值与 Java ConditionKey 接口常量一一对应
#[test]
fn condition_key_strings_match_java_constants() {
    let expect = [
        (ConditionKey::Default, "DEFAULT_KEY"),
        (ConditionKey::For, "FOR_KEY"),
        (ConditionKey::If, "IF_KEY"),
        (ConditionKey::IfTrueCase, "IF_TRUE_CASE_KEY"),
        (ConditionKey::IfFalseCase, "IF_FALSE_CASE_KEY"),
        (ConditionKey::Iterator, "ITERATOR_KEY"),
        (ConditionKey::Do, "DO_KEY"),
        (ConditionKey::Break, "BREAK_KEY"),
        (ConditionKey::Switch, "SWITCH_KEY"),
        (ConditionKey::SwitchTarget, "SWITCH_TARGET_KEY"),
        (ConditionKey::SwitchDefault, "SWITCH_DEFAULT_KEY"),
        (ConditionKey::Pre, "PRE_KEY"),
        (ConditionKey::Finally, "FINALLY_KEY"),
        (ConditionKey::While, "WHILE_KEY"),
        (ConditionKey::Catch, "CATCH_KEY"),
    ];
    assert_eq!(ConditionKey::ALL.len(), 15);
    for (key, s) in expect {
        assert_eq!(key.as_str(), s);
        assert_eq!(ConditionKey::from_key(s), Some(key));
    }
    assert_eq!(ConditionKey::from_key("NO_SUCH_KEY"), None);
}

/// ConditionKey：相等/哈希语义（可作为 HashSet/HashMap 键，对齐 Java 常量唯一性）
#[test]
fn condition_key_eq_and_hash() {
    assert_eq!(ConditionKey::Do, ConditionKey::Do);
    assert_ne!(ConditionKey::Do, ConditionKey::Break);
    let set: HashSet<ConditionKey> = ConditionKey::ALL.into_iter().collect();
    // 15 个键互不重复（哈希语义下仍保持全集）
    assert_eq!(set.len(), 15);
    assert!(set.contains(&ConditionKey::SwitchDefault));
    let cloned = ConditionKey::IfTrueCase;
    assert_eq!(cloned, ConditionKey::IfTrueCase);
}

/// 各 Condition 的 condition_type() 对齐 Java getConditionType()
#[tokio::test]
async fn condition_type_aligns_with_java() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let s = |id: &'static str| Stub::new(id, true, &log);

    let then = ThenCondition::new();
    assert_eq!(then.condition_type(), ConditionTypeEnum::Then);
    assert_eq!(then.condition_type().get_type(), "then");

    let when = WhenCondition::new(vec![s("a")]);
    assert_eq!(when.condition_type(), ConditionTypeEnum::When);

    let if_cond = IfCondition::new(s("if"), s("t"), vec![], None);
    assert_eq!(if_cond.condition_type(), ConditionTypeEnum::If);

    let switch = SwitchCondition::new(s("sw"), vec![s("a")], None);
    assert_eq!(switch.condition_type(), ConditionTypeEnum::Switch);
    assert_eq!(
        switch
            .get_target_list()
            .iter()
            .map(|target| target.id())
            .collect::<Vec<_>>(),
        vec!["a"]
    );

    let for_cond = ForCondition::new(s("for"), false, s("do"), None);
    assert_eq!(for_cond.condition_type(), ConditionTypeEnum::For);

    let while_cond = WhileCondition::new(s("w"), false, s("do"), None);
    assert_eq!(while_cond.condition_type(), ConditionTypeEnum::While);

    let iter_cond = IteratorCondition::new(s("it"), false, s("do"), None);
    assert_eq!(iter_cond.condition_type(), ConditionTypeEnum::Iterator);

    let catch = CatchCondition::new(s("c"), None);
    assert_eq!(catch.condition_type(), ConditionTypeEnum::Catch);

    let pre = PreCondition::new(s("p"));
    assert_eq!(pre.condition_type(), ConditionTypeEnum::Pre);

    let fin = FinallyCondition::new(s("f"));
    assert_eq!(fin.condition_type(), ConditionTypeEnum::Finally);
}

/// 对应 Java IfCondition#executeCondition：isAccess=false 时整个 IF 不执行
#[tokio::test]
async fn if_skipped_when_not_accessible() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let if_item = Stub::new("if", false, &log);
    let true_case = Stub::new("t", true, &log);
    let cond = IfCondition::new(if_item, true_case, vec![], None);
    let (ctx, frame) = ctx_frame();
    let r = cond.execute(&ctx, &frame).await;
    assert!(r.is_ok());
    assert!(log.lock().unwrap().is_empty());
}

/// 对应 Java SwitchCondition#executeCondition：isAccess=false 时整个 SWITCH 不执行
#[tokio::test]
async fn switch_skipped_when_not_accessible() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let sw = Stub::new("sw", false, &log);
    let target = Stub::new("a", true, &log);
    let cond = SwitchCondition::new(sw, vec![target], None);
    let (ctx, frame) = ctx_frame();
    let r = cond.execute(&ctx, &frame).await;
    assert!(r.is_ok());
    assert!(log.lock().unwrap().is_empty());
}

/// 对应 Java ForCondition#executeCondition：isAccess=false 时整个 FOR 不执行
#[tokio::test]
async fn for_skipped_when_not_accessible() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let for_node = Stub::new("for", false, &log);
    let body = Stub::new("do", true, &log);
    let cond = ForCondition::new(for_node, false, body, None);
    let (ctx, frame) = ctx_frame();
    let r = cond.execute(&ctx, &frame).await;
    assert!(r.is_ok());
    assert!(log.lock().unwrap().is_empty());
}

/// 对应 Java WhileCondition#executeCondition：isAccess=false 时整个 WHILE 不执行
#[tokio::test]
async fn while_skipped_when_not_accessible() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let while_item = Stub::new("w", false, &log);
    let body = Stub::new("do", true, &log);
    let cond = WhileCondition::new(while_item, false, body, None);
    let (ctx, frame) = ctx_frame();
    let r = cond.execute(&ctx, &frame).await;
    assert!(r.is_ok());
    assert!(log.lock().unwrap().is_empty());
}

/// 对应 Java IteratorCondition#executeCondition：isAccess=false 时整个 ITERATOR 不执行
#[tokio::test]
async fn iterator_skipped_when_not_accessible() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let it = Stub::new("it", false, &log);
    let body = Stub::new("do", true, &log);
    let cond = IteratorCondition::new(it, false, body, None);
    let (ctx, frame) = ctx_frame();
    let r = cond.execute(&ctx, &frame).await;
    assert!(r.is_ok());
    assert!(log.lock().unwrap().is_empty());
}

/// Java AND/OR 会先调用 Node#isAccess 并缓存 true，随后 Node#execute 不得二次求值。
#[tokio::test]
async fn and_or_reuses_node_access_result_during_boolean_execution() {
    let access_calls = Arc::new(AtomicUsize::new(0));
    let process_calls = Arc::new(AtomicUsize::new(0));
    let node: Arc<dyn Executable> = Arc::new(Node::new(
        NodeRef::new("one-shot-access"),
        Arc::new(OneShotAccessComponent {
            access_calls: Arc::clone(&access_calls),
            process_calls: Arc::clone(&process_calls),
        }),
    ));
    let condition = AndOrCondition::new(BooleanConditionTypeEnum::And, vec![node]);
    let (ctx, frame) = ctx_frame();

    assert_eq!(
        condition
            .execute(&ctx, &frame)
            .await
            .expect("AND 应执行已通过预过滤的节点"),
        Value::Bool(true)
    );
    assert_eq!(access_calls.load(Ordering::SeqCst), 1);
    assert_eq!(process_calls.load(Ordering::SeqCst), 1);
}

/// 对应 Java ParallelStrategyExecutor#filterWhenTaskList：
/// 非 ALL 策略过滤 pre/finally 与 isAccess=false，只并行执行剩余分支。
#[tokio::test]
async fn when_filters_pre_finally_and_inaccessible() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let pre: Arc<dyn Executable> = Arc::new(PreCondition::new(Stub::new("pre", true, &log)));
    let fin: Arc<dyn Executable> = Arc::new(FinallyCondition::new(Stub::new("fin", true, &log)));
    let hidden = Stub::new("hidden", false, &log);
    let a = Stub::new("a", true, &log);
    let b = Stub::new("b", true, &log);
    let mut cond = WhenCondition::new(vec![pre, hidden, a, fin, b]);
    cond.set_parallel_strategy(ParallelStrategyEnum::Specify);
    cond.set_specify_id_set(HashSet::from(["a".to_string(), "b".to_string()]));
    let (ctx, frame) = ctx_frame();
    let r = cond.execute(&ctx, &frame).await;
    assert!(r.is_ok());
    let mut ran = log.lock().unwrap().clone();
    ran.sort_unstable();
    assert_eq!(ran, vec!["a", "b"]);
}

/// Java AllOfParallelExecutor 覆盖 filterAccess，因此 ALL 只过滤 PRE/FINALLY，
/// 不在策略层丢弃 isAccess=false 分支。
#[tokio::test]
async fn when_all_preserves_inaccessible_branches_like_java_override() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let hidden = Stub::new("hidden", false, &log);
    let visible = Stub::new("visible", true, &log);
    let cond = WhenCondition::new(vec![hidden, visible]);
    let (ctx, frame) = ctx_frame();

    cond.execute(&ctx, &frame)
        .await
        .expect("ALL 策略应保留全部普通分支");
    let mut ran = log.lock().unwrap().clone();
    ran.sort_unstable();
    assert_eq!(ran, ["hidden", "visible"]);
}

/// 对应 Java ThenCondition#addExecutable：按类型分流，
/// PRE 先进 pre 列表、FINALLY 进 finally 列表，执行顺序 pre → 主体 → finally
#[tokio::test]
async fn then_add_executable_routes_pre_and_finally() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut cond = ThenCondition::new();
    // 故意乱序 add，验证分流而非插入顺序
    cond.add_executable(Stub::new("m1", true, &log));
    cond.add_executable(Arc::new(FinallyCondition::new(Stub::new(
        "fin", true, &log,
    ))));
    cond.add_executable(Arc::new(PreCondition::new(Stub::new("pre", true, &log))));
    cond.add_executable(Stub::new("m2", true, &log));
    let (ctx, frame) = ctx_frame();
    let r = cond.execute(&ctx, &frame).await;
    assert!(r.is_ok());
    assert_eq!(*log.lock().unwrap(), vec!["pre", "m1", "m2", "fin"]);
}

/// 对应 Java CatchCondition#executeCondition：DO 成功后清除 slot 异常，
/// 整个流程状态为成功（slot.exception 为空）
#[tokio::test]
async fn catch_do_success_clears_slot_exception() {
    let bus = FlowBus::new();
    bus.register(
        "bad",
        cmp(|_| async { Err(LiteflowError::Custom("boom".into())) }),
    );
    bus.register(
        "handle",
        cmp(|ctx| async move {
            // catch 住时 slot 上已有异常（对应 Java Condition.execute 的事先 setException）
            ctx.set_data("caught", json!(true));
            Ok(Value::Null)
        }),
    );
    bus.add_chain("c1", "CATCH(bad).DO(handle)").unwrap();
    bus.add_chain("c2", "CATCH(bad)").unwrap();

    let resp = bus.execute("c1").await;
    assert!(resp.is_success());
    assert_eq!(resp.data("caught"), Some(json!(true)));
    // DO 成功后 slot 异常被清除
    assert_eq!(resp.slot_exception(), None);

    // 无 DO 时异常继续上抛，slot 异常保留
    let resp2 = bus.execute("c2").await;
    assert!(!resp2.is_success());
    assert!(resp2.slot_exception().is_some());
}

/// 探针：直接构造 CatchCondition 验证 execute 路径覆盖
#[tokio::test]
async fn probe_catch_direct_execute() {
    let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    struct FailStub;
    #[async_trait::async_trait]
    impl Executable for FailStub {
        async fn execute(&self, _ctx: &Ctx, _frame: &Frame) -> LFResult<Value> {
            Err(LiteflowError::Custom("boom".into()))
        }
        fn id(&self) -> &str {
            "fail"
        }
    }
    let catch = CatchCondition::new(Arc::new(FailStub), Some(Stub::new("dook", true, &log)));
    let (ctx, frame) = ctx_frame();
    let r = catch.execute_condition(&ctx, &frame).await;
    assert!(r.is_ok());
    assert_eq!(*log.lock().unwrap(), vec!["dook"]);
    assert_eq!(ctx.inner.exception.lock().unwrap().as_ref(), None);
    // 无 DO：异常继续抛出
    let catch2 = CatchCondition::new(Arc::new(FailStub), None);
    let (ctx2, frame2) = ctx_frame();
    let r2 = catch2.execute_condition(&ctx2, &frame2).await;
    assert!(r2.is_err());
}
