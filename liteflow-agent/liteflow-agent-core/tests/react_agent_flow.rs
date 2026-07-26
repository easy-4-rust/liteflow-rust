//! LiteFlow 与 AgentScope-Rust ReActAgent 的真实执行链测试。

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agentscope_core::model::{
    ChatResponse, GenerateOptions, ModelCapabilities, ModelError, ToolSchema,
};
use agentscope_core::{Model, Msg};
use futures::Stream;
use liteflow_agent_core::{
    AgentConfig, AgentError, AgentEventType, MemoryStorageMode, ReActAgentComponent,
};
use liteflow_core::{ExecuteOption, FlowBus, FlowEvent, listener};
use serde_json::{Value, json};

/// 记录真实模型调用次数并返回确定性文本的测试模型。
struct CountingModel {
    calls: AtomicUsize,
    delay: Duration,
}

impl CountingModel {
    fn new(delay: Duration) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            delay,
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Model for CountingModel {
    fn name(&self) -> &str {
        "liteflow-counting-model"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::basic()
    }

    fn stream(
        &self,
        _messages: &[Msg],
        _tools: &[ToolSchema],
        _options: Option<&GenerateOptions>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponse, ModelError>> + Send>> {
        let invocation = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let delay = self.delay;
        Box::pin(futures::stream::once(async move {
            tokio::time::sleep(delay).await;
            Ok(ChatResponse::from_text(format!("reply-{invocation}")))
        }))
    }
}

#[tokio::test]
async fn react_agent_runs_in_for_chain_and_reuses_conversation_session() {
    let model = Arc::new(CountingModel::new(Duration::from_millis(1)));
    let component = Arc::new(
        ReActAgentComponent::builder("agent", model.clone())
            .build()
            .expect("ReAct Agent component should build"),
    );
    let bus = FlowBus::new();
    bus.register_arc("agent", component.clone());
    bus.add_chain("agent_loop", "FOR(2).DO(agent)")
        .expect("agent loop chain should build");

    let events = Arc::new(Mutex::new(Vec::<FlowEvent>::new()));
    let captured_events = events.clone();
    let option = ExecuteOption::of()
        .request_id("agent-request-1")
        .conversation_id("conversation-a")
        .event_listener(Arc::new(listener(move |event| {
            captured_events
                .lock()
                .expect("event buffer lock")
                .push(event.clone());
        })));

    let response = bus
        .execute_with_option("agent_loop", json!({"prompt": "你好"}), option)
        .await;

    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("agent_result"), Some(json!("reply-2")));
    assert_eq!(model.call_count(), 2);
    assert_eq!(component.sessions().session_count(), 1);

    let captured = events.lock().expect("event buffer lock");
    assert_eq!(captured.len(), 4);
    assert_eq!(
        captured
            .iter()
            .filter(|event| event.event_type == AgentEventType::REASONING)
            .count(),
        2
    );
    assert_eq!(
        captured
            .iter()
            .filter(|event| event.event_type == AgentEventType::RESULT && event.last)
            .count(),
        2
    );
    assert!(captured.iter().all(|event| {
        event.chain_id.as_deref() == Some("agent_loop")
            && event.node_id.as_deref() == Some("agent")
            && event.request_id.as_deref() == Some("agent-request-1")
            && event.conversation_id.as_deref() == Some("conversation-a")
    }));
    drop(captured);

    let second_response = bus
        .execute_with_option(
            "agent_loop",
            Value::String("继续".to_string()),
            ExecuteOption::of().conversation_id("conversation-a"),
        )
        .await;
    assert!(second_response.is_success(), "{}", second_response.message);
    assert_eq!(second_response.data("agent_result"), Some(json!("reply-4")));
    assert_eq!(model.call_count(), 4);
    assert_eq!(component.sessions().session_count(), 1);
}

#[tokio::test]
async fn same_conversation_is_serialized_inside_parallel_when() {
    let model = Arc::new(CountingModel::new(Duration::from_millis(20)));
    let component = Arc::new(
        ReActAgentComponent::builder("parallel_agent", model.clone())
            .build()
            .expect("ReAct Agent component should build"),
    );
    let bus = FlowBus::new();
    bus.register_arc("parallel_agent", component.clone());
    bus.add_chain(
        "parallel_agent_chain",
        "WHEN(parallel_agent, parallel_agent)",
    )
    .expect("parallel agent chain should build");

    let response = bus
        .execute_with_option(
            "parallel_agent_chain",
            json!({"prompt": "并行执行"}),
            ExecuteOption::of().conversation_id("conversation-parallel"),
        )
        .await;

    assert!(
        response.is_success(),
        "同一会话应由 AgentSessionEntry gate 串行化，不能出现 AlreadyRunning: {}",
        response.message
    );
    assert_eq!(model.call_count(), 2);
    assert_eq!(component.sessions().session_count(), 1);
}

#[tokio::test]
async fn local_file_memory_uses_real_agentscope_json_session() {
    let directory = tempfile::tempdir().expect("应创建临时工作区");
    let mut config = AgentConfig::default();
    config.workspace.root = Some(directory.path().to_string_lossy().into_owned());
    config.session.memory.mode = MemoryStorageMode::LocalFile;

    let model = Arc::new(CountingModel::new(Duration::from_millis(1)));
    let component = Arc::new(
        ReActAgentComponent::builder("persistent_agent", model)
            .config(config)
            .build()
            .expect("LOCAL_FILE 应自动构造 AgentScope JsonSession"),
    );
    let bus = FlowBus::new();
    bus.register_arc("persistent_agent", component);
    bus.add_chain("persistent_chain", "THEN(persistent_agent)")
        .expect("应创建持久化 Agent 链");

    let response = bus
        .execute_with_option(
            "persistent_chain",
            json!({"prompt": "保存会话"}),
            ExecuteOption::of().conversation_id("persistent-conversation"),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);

    let session_root = directory.path().join(".agent-session");
    assert!(session_root.is_dir());
    let persisted_entries = std::fs::read_dir(&session_root)
        .expect("应读取 Session 根目录")
        .count();
    assert!(
        persisted_entries > 0,
        "真实 AgentScope Session 应写入会话目录"
    );
}

#[tokio::test]
async fn session_config_enforces_lru_limit_and_idle_cleanup() {
    let mut config = AgentConfig::default();
    config.session.max_sessions = 1;
    config.session.idle_timeout = Duration::from_millis(5);
    config.session.cleanup_interval = Duration::from_millis(1);

    let model = Arc::new(CountingModel::new(Duration::ZERO));
    let component = Arc::new(
        ReActAgentComponent::builder("managed_agent", model)
            .config(config)
            .build()
            .expect("应构建带会话生命周期配置的 Agent"),
    );
    let bus = FlowBus::new();
    bus.register_arc("managed_agent", component.clone());
    bus.add_chain("managed_chain", "THEN(managed_agent)")
        .expect("应创建会话管理链");

    for conversation_id in ["first", "second"] {
        let response = bus
            .execute_with_option(
                "managed_chain",
                json!({"prompt": conversation_id}),
                ExecuteOption::of().conversation_id(conversation_id),
            )
            .await;
        assert!(response.is_success(), "{}", response.message);
    }
    assert_eq!(component.sessions().session_count(), 1);

    tokio::time::sleep(Duration::from_millis(25)).await;
    let response = bus
        .execute_with_option(
            "managed_chain",
            json!({"prompt": "third"}),
            ExecuteOption::of().conversation_id("third"),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(component.sessions().session_count(), 1);
}

#[test]
fn remote_memory_backends_require_explicit_agentscope_session() {
    for mode in [MemoryStorageMode::Redis, MemoryStorageMode::Mysql] {
        let mut config = AgentConfig::default();
        config.session.memory.mode = mode;
        let model = Arc::new(CountingModel::new(Duration::ZERO));
        let result = ReActAgentComponent::builder("remote_agent", model)
            .config(config)
            .build();
        assert!(matches!(
            result,
            Err(AgentError::SessionBackendRequiresInjection(actual)) if actual == mode
        ));
    }
}
