//! LiteFlow 与 AgentScope-Rust ReActAgent 的真实执行链测试。

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agentscope_core::message::{ChatUsage, ToolUseBlock};
use agentscope_core::model::{
    ChatResponse, GenerateOptions, ModelCapabilities, ModelError, ToolSchema,
};
use agentscope_core::session::{InMemorySession, Session};
use agentscope_core::{ContentBlock, Model, Msg};
use futures::Stream;
use liteflow_agent_core::{
    AgentConfig, AgentEventType, AgentSessionFactoryRegistry, MemoryStorageMode,
    MysqlAgentSessionFactory, ReActAgentComponent, RedisAgentSessionFactory,
};
use liteflow_core::{ExecuteOption, FlowBus, FlowEvent, listener};
use serde_json::{Value, json};

#[derive(Clone)]
struct CapturedOutput(Arc<Mutex<Vec<u8>>>);

struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedOutput {
    type Writer = CapturedWriter;

    fn make_writer(&'a self) -> Self::Writer {
        CapturedWriter(self.0.clone())
    }
}

impl Write for CapturedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("日志捕获缓冲区锁")
            .extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// 记录真实模型调用次数并返回确定性文本的测试模型。
struct CountingModel {
    calls: AtomicUsize,
    delay: Duration,
    tool_names: Mutex<Vec<String>>,
}

/// 上报确定性 usage 的测试模型，用于验证 invocation 上下文中的完整用量。
struct UsageReportingModel;

impl Model for UsageReportingModel {
    fn name(&self) -> &str {
        "liteflow-usage-reporting-model"
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
        Box::pin(futures::stream::once(async {
            Ok(ChatResponse::from_text_with_usage(
                "usage-reply",
                ChatUsage {
                    input_tokens: 12,
                    output_tokens: 4,
                    cached_tokens: 2,
                    time: 0.25,
                },
            ))
        }))
    }
}

impl CountingModel {
    fn new(delay: Duration) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            delay,
            tool_names: Mutex::new(Vec::new()),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn tool_names(&self) -> Vec<String> {
        self.tool_names.lock().expect("tool names lock").clone()
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
        tools: &[ToolSchema],
        _options: Option<&GenerateOptions>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponse, ModelError>> + Send>> {
        let invocation = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        *self.tool_names.lock().expect("tool names lock") =
            tools.iter().map(|tool| tool.name.clone()).collect();
        let delay = self.delay;
        Box::pin(futures::stream::once(async move {
            tokio::time::sleep(delay).await;
            Ok(ChatResponse::from_text(format!("reply-{invocation}")))
        }))
    }
}

#[tokio::test]
async fn invocation_context_exposes_usage_to_reply_handler_and_is_always_removed() {
    let workspace_root = tempfile::tempdir().expect("应创建上下文测试工作区");
    let mut config = AgentConfig::default();
    config.workspace.root = Some(workspace_root.path().to_string_lossy().into_owned());

    let observed = Arc::new(Mutex::new(None));
    let observed_by_handler = Arc::clone(&observed);
    let component = Arc::new(
        ReActAgentComponent::builder("context_agent", Arc::new(UsageReportingModel))
            .agent_key("stable-agent")
            .config(config)
            .enable_workspace_file_tools(false)
            .enable_shell_tool(false)
            .handle_reply(move |runtime_context, reply| {
                let usage = runtime_context
                    .chat_usage()
                    .expect("handle_reply 中应观察到完整 usage");
                assert_eq!(runtime_context.conversation_id(), "context-conversation");
                assert_eq!(runtime_context.agent_key(), "stable-agent");
                assert!(runtime_context.workspace_dir().is_none());
                assert_eq!(runtime_context.cmp_context().node_id(), "context_agent");
                assert_eq!(reply.get_text_content(), "usage-reply");
                assert_eq!(usage.input_tokens, 12);
                assert_eq!(usage.output_tokens, 4);
                assert_eq!(usage.cached_tokens, 2);
                assert!((usage.time - 0.25).abs() < f64::EPSILON);
                *observed_by_handler.lock().expect("运行时上下文捕获锁") =
                    Some(runtime_context.cmp_context().clone());
                Ok(())
            })
            .build()
            .expect("应构建带回复回调的 Agent"),
    );
    let bus = FlowBus::new();
    bus.register_arc("context_agent", component.clone());
    bus.add_chain("context_chain", "THEN(context_agent)")
        .expect("应创建上下文测试链");

    let response = bus
        .execute_with_option(
            "context_chain",
            json!({"prompt": "统计用量"}),
            ExecuteOption::of().conversation_id("context-conversation"),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("agent_result"), Some(json!("usage-reply")));
    let completed_context = observed
        .lock()
        .expect("运行时上下文捕获锁")
        .clone()
        .expect("handle_reply 应被调用");
    assert!(
        component.runtime_context(&completed_context).is_none(),
        "成功调用结束后必须移除 invocation 上下文附件"
    );

    let failed_context = Arc::new(Mutex::new(None));
    let failed_context_by_prompt = Arc::clone(&failed_context);
    let failed_component = Arc::new(
        ReActAgentComponent::builder("failed_context_agent", Arc::new(UsageReportingModel))
            .enable_workspace_file_tools(false)
            .enable_shell_tool(false)
            .user_prompt(move |context| {
                *failed_context_by_prompt.lock().expect("失败上下文捕获锁") = Some(context.clone());
                Ok("   ".to_string())
            })
            .build()
            .expect("应构建空提示词测试 Agent"),
    );
    bus.register_arc("failed_context_agent", failed_component.clone());
    bus.add_chain("failed_context_chain", "THEN(failed_context_agent)")
        .expect("应创建失败上下文测试链");

    let failed_response = bus
        .execute_with_option(
            "failed_context_chain",
            json!({"prompt": "不会使用"}),
            ExecuteOption::of().conversation_id("failed-context"),
        )
        .await;
    assert!(!failed_response.is_success());
    let completed_failed_context = failed_context
        .lock()
        .expect("失败上下文捕获锁")
        .clone()
        .expect("提示词解析器应观察到已绑定上下文");
    assert!(
        failed_component
            .runtime_context(&completed_failed_context)
            .is_none(),
        "提示词校验失败后也必须移除 invocation 上下文附件"
    );
}

/// 首轮请求真实技能加载工具、次轮返回文本的确定性测试模型。
struct SkillCallingModel {
    calls: AtomicUsize,
    tool_names: Mutex<Vec<String>>,
}

impl SkillCallingModel {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            tool_names: Mutex::new(Vec::new()),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn tool_names(&self) -> Vec<String> {
        self.tool_names.lock().expect("tool names lock").clone()
    }
}

impl Model for SkillCallingModel {
    fn name(&self) -> &str {
        "liteflow-skill-calling-model"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::basic()
    }

    fn stream(
        &self,
        _messages: &[Msg],
        tools: &[ToolSchema],
        _options: Option<&GenerateOptions>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponse, ModelError>> + Send>> {
        let invocation = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        *self.tool_names.lock().expect("tool names lock") =
            tools.iter().map(|tool| tool.name.clone()).collect();
        Box::pin(futures::stream::once(async move {
            if invocation == 1 {
                Ok(ChatResponse {
                    content: vec![ContentBlock::ToolUse(ToolUseBlock::new(
                        "load-beta",
                        "load_skill_through_path",
                        HashMap::from([
                            ("skillId".to_string(), json!("beta")),
                            ("path".to_string(), json!("SKILL.md")),
                        ]),
                    ))],
                    usage: Some(ChatUsage {
                        input_tokens: 10,
                        output_tokens: 2,
                        cached_tokens: 1,
                        time: 0.1,
                    }),
                    finish_reason: Some("tool_calls".to_string()),
                    ..ChatResponse::default()
                })
            } else if invocation == 2 {
                Ok(ChatResponse {
                    content: vec![ContentBlock::text("skill-loaded")],
                    usage: Some(ChatUsage {
                        input_tokens: 20,
                        output_tokens: 3,
                        cached_tokens: 2,
                        time: 0.2,
                    }),
                    ..ChatResponse::default()
                })
            } else {
                Ok(ChatResponse::from_text("no-skill"))
            }
        }))
    }
}

#[tokio::test]
async fn enabled_skills_bind_real_agentscope_load_tool() {
    let skills_root = tempfile::tempdir().expect("应创建 Skills 临时目录");
    let workspace_root = tempfile::tempdir().expect("应创建工作区临时目录");
    for (directory_name, skill_name) in [("alpha", "Alpha"), ("beta", "Beta")] {
        let skill_directory = skills_root.path().join(directory_name);
        fs::create_dir_all(&skill_directory).expect("应创建技能目录");
        fs::write(
            skill_directory.join("SKILL.md"),
            format!(
                "---\nname: {skill_name}\ndescription: {skill_name} 测试技能\n---\n\n# {skill_name}"
            ),
        )
        .expect("应写入 SKILL.md");
    }

    let mut config = AgentConfig::default();
    config.skills.enabled = true;
    config.skills.path = skills_root.path().to_string_lossy().into_owned();
    config.workspace.root = Some(workspace_root.path().to_string_lossy().into_owned());
    let model = Arc::new(SkillCallingModel::new());
    let component = Arc::new(
        ReActAgentComponent::builder("skills_agent", model.clone())
            .config(config)
            .skills(["Beta"])
            .enable_workspace_file_tools(false)
            .enable_shell_tool(false)
            .build()
            .expect("应构建 Skills Agent"),
    );
    let bus = FlowBus::new();
    bus.register_arc("skills_agent", component.clone());
    bus.add_chain("skills_chain", "THEN(skills_agent)")
        .expect("应创建 Skills 链");

    let events = Arc::new(Mutex::new(Vec::<FlowEvent>::new()));
    let captured_events = Arc::clone(&events);
    let response = bus
        .execute_with_option(
            "skills_chain",
            json!({"prompt": "使用技能"}),
            ExecuteOption::of()
                .conversation_id("skills-conversation")
                .event_listener(Arc::new(listener(move |event| {
                    captured_events
                        .lock()
                        .expect("技能事件缓冲区锁")
                        .push(event.clone());
                }))),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("agent_result"), Some(json!("skill-loaded")));
    assert_eq!(model.call_count(), 2, "应经历工具调用与最终回答两轮推理");
    assert_eq!(
        model.tool_names(),
        vec!["load_skill_through_path"],
        "AgentScope SkillBox 必须把真实技能加载工具绑定到 ReAct Toolkit"
    );
    assert_eq!(
        component.sessions().used_skills("skills-conversation"),
        vec!["Beta"],
        "会话必须保留当前 invocation 成功加载的技能"
    );
    let usage = component
        .sessions()
        .chat_usage("skills-conversation")
        .expect("两轮模型响应都上报 usage");
    assert_eq!(usage.input_tokens, 30);
    assert_eq!(usage.output_tokens, 5);
    assert_eq!(usage.cached_tokens, 3);
    assert!((usage.time - 0.3).abs() < f64::EPSILON);
    assert_eq!(
        component.sessions().chat_usage_steps("skills-conversation"),
        2
    );
    assert!(
        events
            .lock()
            .expect("技能事件缓冲区锁")
            .iter()
            .any(|event| event.event_type == AgentEventType::TOOL_RESULT),
        "真实技能加载工具调用必须发布 agent.tool_result 流事件"
    );
    assert!(
        workspace_root.path().join("skills-conversation").is_dir(),
        "启用 Skills 时应为会话创建真实代码执行工作区"
    );

    let second = bus
        .execute_with_option(
            "skills_chain",
            json!({"prompt": "本轮不加载技能"}),
            ExecuteOption::of().conversation_id("skills-conversation"),
        )
        .await;
    assert!(second.is_success(), "{}", second.message);
    assert!(
        component
            .sessions()
            .used_skills("skills-conversation")
            .is_empty(),
        "每次 invocation 前必须清空上一轮技能记录"
    );
    assert!(
        component
            .sessions()
            .chat_usage("skills-conversation")
            .is_none(),
        "下一 invocation 未上报 usage 时不得残留上一轮 token"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn logging_config_and_builder_override_control_real_react_output() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_writer(CapturedOutput(output.clone()))
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("应安装测试日志订阅器");
    {
        let bus = FlowBus::new();
        let enabled_model = Arc::new(CountingModel::new(Duration::ZERO));
        bus.register_arc(
            "logging_enabled_agent",
            Arc::new(
                ReActAgentComponent::builder("logging_enabled_agent", enabled_model)
                    .build()
                    .expect("默认日志开启时应构建 Agent"),
            ),
        );
        bus.add_chain("logging_enabled_chain", "THEN(logging_enabled_agent)")
            .expect("应创建默认日志链");

        let mut disabled_config = AgentConfig::default();
        disabled_config.logging.react_enabled = false;
        let disabled_model = Arc::new(CountingModel::new(Duration::ZERO));
        bus.register_arc(
            "logging_disabled_agent",
            Arc::new(
                ReActAgentComponent::builder("logging_disabled_agent", disabled_model)
                    .config(disabled_config.clone())
                    .build()
                    .expect("日志关闭时应构建 Agent"),
            ),
        );
        bus.add_chain("logging_disabled_chain", "THEN(logging_disabled_agent)")
            .expect("应创建关闭日志链");

        let override_model = Arc::new(CountingModel::new(Duration::ZERO));
        bus.register_arc(
            "logging_override_agent",
            Arc::new(
                ReActAgentComponent::builder("logging_override_agent", override_model)
                    .config(disabled_config)
                    .enable_react_logging(true)
                    .build()
                    .expect("组件覆盖开启日志时应构建 Agent"),
            ),
        );
        bus.add_chain("logging_override_chain", "THEN(logging_override_agent)")
            .expect("应创建覆盖日志链");

        for (chain_id, conversation_id) in [
            ("logging_enabled_chain", "logging-on"),
            ("logging_disabled_chain", "logging-off"),
            ("logging_override_chain", "logging-override"),
        ] {
            let response = bus
                .execute_with_option(
                    chain_id,
                    json!({"prompt": conversation_id}),
                    ExecuteOption::of().conversation_id(conversation_id),
                )
                .await;
            assert!(response.is_success(), "{}", response.message);
        }
    }

    let logs = String::from_utf8(output.lock().expect("日志捕获缓冲区锁").clone())
        .expect("日志应为 UTF-8");
    assert!(
        logs.contains("[agent:reason][logging-on:logging_enabled_agent]"),
        "未捕获默认开启日志：{logs}"
    );
    assert!(!logs.contains("[agent:reason][logging-off:logging_disabled_agent]"));
    assert!(logs.contains("[agent:reason][logging-override:logging_override_agent]"));
    assert!(logs.contains("messages="));
    assert!(logs.contains("text=reply-1"));
}

#[tokio::test]
async fn configured_workspace_tools_are_registered_per_conversation() {
    let directory = tempfile::tempdir().expect("应创建临时工作区");
    let mut config = AgentConfig::default();
    config.workspace.root = Some(directory.path().to_string_lossy().into_owned());
    let model = Arc::new(CountingModel::new(Duration::ZERO));
    let component = Arc::new(
        ReActAgentComponent::builder("workspace_agent", model.clone())
            .config(config)
            .build()
            .expect("应构建工作区 Agent"),
    );
    let bus = FlowBus::new();
    bus.register_arc("workspace_agent", component);
    bus.add_chain("workspace_chain", "THEN(workspace_agent)")
        .expect("应创建 Agent 链");

    let response = bus
        .execute_with_option(
            "workspace_chain",
            json!({"prompt": "列出文件"}),
            ExecuteOption::of().conversation_id("客户 / 会话"),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);

    let mut names = model.tool_names();
    names.sort();
    assert_eq!(
        names,
        vec![
            "delete_file",
            "execute_shell_command",
            "list_files",
            "read_file",
            "write_file"
        ]
    );
    assert!(
        std::fs::read_dir(directory.path())
            .expect("应读取根目录")
            .any(|entry| entry.expect("应读取会话目录项").path().is_dir()),
        "首次执行应真实创建经过安全转义的会话目录"
    );
}

#[tokio::test]
async fn workspace_tools_can_be_explicitly_disabled() {
    let directory = tempfile::tempdir().expect("应创建临时工作区");
    let mut config = AgentConfig::default();
    config.workspace.root = Some(directory.path().to_string_lossy().into_owned());
    let model = Arc::new(CountingModel::new(Duration::ZERO));
    let component = Arc::new(
        ReActAgentComponent::builder("workspace_disabled_agent", model.clone())
            .config(config)
            .enable_workspace_file_tools(false)
            .enable_shell_tool(false)
            .build()
            .expect("禁用文件工具后仍应构建 Agent"),
    );
    let bus = FlowBus::new();
    bus.register_arc("workspace_disabled_agent", component);
    bus.add_chain("workspace_disabled_chain", "THEN(workspace_disabled_agent)")
        .expect("应创建 Agent 链");

    let response = bus
        .execute_with_option(
            "workspace_disabled_chain",
            json!({"prompt": "不使用工作区"}),
            ExecuteOption::of(),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    assert!(model.tool_names().is_empty());
}

#[tokio::test]
async fn disabled_shell_mode_does_not_register_shell_tool() {
    let directory = tempfile::tempdir().expect("应创建临时工作区");
    let mut config = AgentConfig::default();
    config.workspace.root = Some(directory.path().to_string_lossy().into_owned());
    config.shell.mode = liteflow_agent_core::ShellMode::Disabled;
    let model = Arc::new(CountingModel::new(Duration::ZERO));
    let component = Arc::new(
        ReActAgentComponent::builder("shell_disabled_agent", model.clone())
            .config(config)
            .build()
            .expect("DISABLED 模式仍应构建 Agent"),
    );
    let bus = FlowBus::new();
    bus.register_arc("shell_disabled_agent", component);
    bus.add_chain("shell_disabled_chain", "THEN(shell_disabled_agent)")
        .expect("应创建 Agent 链");

    let response = bus
        .execute_with_option(
            "shell_disabled_chain",
            json!({"prompt": "不执行命令"}),
            ExecuteOption::of().conversation_id("shell-disabled"),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    let names = model.tool_names();
    assert!(!names.iter().any(|name| name == "execute_shell_command"));
    assert_eq!(names.len(), 4, "工作区文件工具仍应保留");
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
    assert!(component.sessions().contains("conversation-a"));
    let cached_session = component
        .sessions()
        .session("conversation-a")
        .expect("会话应已缓存");
    assert_eq!(cached_session.conversation_id(), "conversation-a");
    assert_eq!(cached_session.agent_key(), "agent");
    assert_eq!(cached_session.cache_key(), "conversation-a__agent");

    {
        let captured = events.lock().expect("event buffer lock");
        assert!(
            captured
                .iter()
                .filter(|event| event.event_type == AgentEventType::REASONING)
                .count()
                >= 2,
            "每次调用都应发布 AgentScope 的真实 reasoning 流事件"
        );
        assert_eq!(
            captured
                .iter()
                .filter(|event| event.event_type == AgentEventType::RESULT && event.last)
                .count(),
            2
        );
        assert!(
            captured
                .iter()
                .filter(|event| event.event_type == AgentEventType::RESULT)
                .all(|event| event.data.is_some()),
            "流式最终事件必须携带可序列化的 AgentScope 原始事件"
        );
        assert!(captured.iter().all(|event| {
            event.chain_id.as_deref() == Some("agent_loop")
                && event.node_id.as_deref() == Some("agent")
                && event.request_id.as_deref() == Some("agent-request-1")
                && event.conversation_id.as_deref() == Some("conversation-a")
        }));
    }

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
    let workspace_root = tempfile::tempdir().expect("应创建会话清理工作区");
    let mut config = AgentConfig::default();
    config.workspace.root = Some(workspace_root.path().to_string_lossy().into_owned());
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
    assert!(
        workspace_root.path().join("first").is_dir(),
        "LRU 淘汰只清理内存缓存，不应删除历史工作区"
    );
    assert!(workspace_root.path().join("second").is_dir());

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
    assert!(
        !workspace_root.path().join("second").exists(),
        "空闲超时淘汰应按 cleanupOnSessionExpire 删除无兄弟会话的工作区"
    );
    assert!(
        workspace_root.path().join("first").is_dir(),
        "先前的 LRU 淘汰工作区必须继续保留"
    );
}

#[tokio::test]
async fn shared_workspace_survives_one_agent_eviction_and_is_deleted_by_last_owner() {
    let workspace_root = tempfile::tempdir().expect("应创建跨 Agent 工作区");
    let workspace_root_value = workspace_root.path().to_string_lossy().into_owned();

    let mut first_config = AgentConfig::default();
    first_config.workspace.root = Some(workspace_root_value.clone());
    first_config.session.idle_timeout = Duration::from_millis(5);
    first_config.session.cleanup_interval = Duration::from_millis(1);

    let mut second_config = AgentConfig::default();
    second_config.workspace.root = Some(workspace_root_value);
    second_config.session.idle_timeout = Duration::from_secs(3_600);
    second_config.workspace.cleanup_on_jvm_shutdown = true;

    let first_component = Arc::new(
        ReActAgentComponent::builder(
            "workspace_agent_a",
            Arc::new(CountingModel::new(Duration::ZERO)),
        )
        .agent_key("agent-a")
        .config(first_config)
        .build()
        .expect("应构建第一个共享工作区 Agent"),
    );
    let second_component = Arc::new(
        ReActAgentComponent::builder(
            "workspace_agent_b",
            Arc::new(CountingModel::new(Duration::ZERO)),
        )
        .agent_key("agent-b")
        .config(second_config)
        .build()
        .expect("应构建第二个共享工作区 Agent"),
    );

    let bus = FlowBus::new();
    bus.register_arc("workspace_agent_a", first_component.clone());
    bus.register_arc("workspace_agent_b", second_component.clone());
    bus.add_chain(
        "shared_workspace_chain",
        "THEN(workspace_agent_a, workspace_agent_b)",
    )
    .expect("应创建双 Agent 链");
    bus.add_chain("first_agent_chain", "THEN(workspace_agent_a)")
        .expect("应创建单 Agent 清理触发链");

    let response = bus
        .execute_with_option(
            "shared_workspace_chain",
            json!({"prompt": "共享工作区"}),
            ExecuteOption::of().conversation_id("shared-conversation"),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);

    let shared_workspace = workspace_root.path().join("shared-conversation");
    assert!(shared_workspace.is_dir());
    // 双 Agent 响应的 CmpStep 同时持有两个组件实例；完成结果断言后释放它，避免
    // 旧响应把第二个会话管理器延长到测试作用域末尾。
    drop(response);
    tokio::time::sleep(Duration::from_millis(25)).await;

    let response = bus
        .execute_with_option(
            "first_agent_chain",
            json!({"prompt": "触发第一个 Agent 的空闲清理"}),
            ExecuteOption::of().conversation_id("next-conversation"),
        )
        .await;
    assert!(response.is_success(), "{}", response.message);
    assert!(
        shared_workspace.is_dir(),
        "第一个 agentKey 过期时，第二个 Agent 仍持有的共享工作区不得被删除"
    );
    assert!(
        second_component.sessions().contains("shared-conversation"),
        "第二个 Agent 的会话必须继续存活"
    );

    // LiteflowResponse 的 CmpStep 按 Java 语义保存组件实例；响应仍存活时组件就不能
    // Drop。先释放响应，再验证最后一个组件持有者退出后的工作区清理。
    drop(response);
    drop(bus);
    drop(first_component);
    drop(second_component);
    assert!(
        !shared_workspace.exists(),
        "最后一个配置 shutdown 清理的工作区持有者退出后应删除共享目录"
    );
}

#[test]
fn remote_memory_backends_resolve_named_agentscope_sessions() {
    let cases = [
        (MemoryStorageMode::Redis, "redisClient"),
        (MemoryStorageMode::Mysql, "agentDataSource"),
    ];

    for (mode, bean_name) in cases {
        let session: Arc<dyn Session> = Arc::new(InMemorySession::new());
        let mut config = AgentConfig::default();
        config.session.memory.mode = mode;

        match mode {
            MemoryStorageMode::Redis => {
                config.session.memory.redis.bean_name = Some(bean_name.to_string());
                RedisAgentSessionFactory::register_session(bean_name, session.clone())
                    .expect("Redis Session 应能按 beanName 注册");
            }
            MemoryStorageMode::Mysql => {
                config.session.memory.mysql.data_source_bean_name = Some(bean_name.to_string());
                MysqlAgentSessionFactory::register_session(bean_name, session.clone())
                    .expect("MySQL Session 应能按 DataSource 名称注册");
            }
            _ => unreachable!("测试仅覆盖两个远端记忆后端"),
        }

        let resolved = AgentSessionFactoryRegistry::new()
            .create_session(&config)
            .expect("命名后端应能从配置解析")
            .expect("远端模式应返回 Session");
        assert!(
            Arc::ptr_eq(&session, &resolved),
            "工厂必须返回宿主注册的同一真实 Session"
        );

        match mode {
            MemoryStorageMode::Redis => {
                RedisAgentSessionFactory::unregister_session(bean_name);
            }
            MemoryStorageMode::Mysql => {
                MysqlAgentSessionFactory::unregister_session(bean_name);
            }
            _ => unreachable!("测试仅覆盖两个远端记忆后端"),
        }
    }
}

#[test]
fn remote_memory_backends_reject_missing_named_resources() {
    let mut redis_config = AgentConfig::default();
    redis_config.session.memory.mode = MemoryStorageMode::Redis;
    let redis_error = match AgentSessionFactoryRegistry::new().create_session(&redis_config) {
        Err(error) => error,
        Ok(_) => panic!("REDIS 模式必须配置 beanName"),
    };
    assert!(redis_error.to_string().contains("redis.beanName"));

    let mut mysql_config = AgentConfig::default();
    mysql_config.session.memory.mode = MemoryStorageMode::Mysql;
    let mysql_error = match AgentSessionFactoryRegistry::new().create_session(&mysql_config) {
        Err(error) => error,
        Ok(_) => panic!("MYSQL 模式必须配置 dataSourceBeanName"),
    };
    assert!(mysql_error.to_string().contains("mysql.dataSourceBeanName"));
}
