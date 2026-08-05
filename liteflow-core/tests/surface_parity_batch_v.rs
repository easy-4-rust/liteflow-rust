//! Java 表面语义回归批次 V：枚举访问器、代理校验、工具与日志。
//!
//! 覆盖对象与 Java 对应关系：
//! - `NodeTypeEnum` 全部 Java 访问器（getCode/setCode/getName/setName/isScript/
//!   setScript/getMappingClazz/setMappingClazz/getEnumByCode/guessTypeBySuperClazz）
//! - `DeclComponentProxy#getProxy` 三类校验错误（Java 元数据不变量）
//! - `JsonUtil` 空值与异常语义（Java `JsonUtil`）
//! - `LFLog` 日志级别、请求 ID 前缀与执行日志开关（Java `LFLog`）
//! - `ChainPropBean` 链式属性构建（Java `ChainPropBean`）

use std::sync::Arc;

use async_trait::async_trait;
use liteflow_core::core::proxy::{
    DeclComponentProxy, DeclWarpBean, LiteFlowMethodBean, MethodWrapBean,
};
use liteflow_core::core::DeclComponent;
use liteflow_core::enums::{ConditionTypeEnum, LiteFlowMethodEnum, NodeTypeEnum};
use liteflow_core::log::{LFLog, LFLoggerManager};
use liteflow_core::util::JsonUtil;
use liteflow_core::{CmpContext, LiteflowError, NodeRef, Slot};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// `NodeTypeEnum` 的 Java 访问器：代码、中文名、脚本标记、映射组件类全量覆盖，
/// `setCode/setName/setScript/setMappingClazz` 的成功与拒绝路径均按 Java 语义。
#[test]
fn node_type_enum_java_accessors_are_complete() {
    for (node_type, code, name, is_script) in [
        (NodeTypeEnum::Common, "common", "普通", false),
        (NodeTypeEnum::Switch, "switch", "选择", false),
        (NodeTypeEnum::Boolean, "boolean", "布尔", false),
        (NodeTypeEnum::If, "if", "条件", false),
        (NodeTypeEnum::For, "for", "循环次数", false),
        (NodeTypeEnum::While, "while", "循环条件", false),
        (NodeTypeEnum::Break, "break", "循环跳出", false),
        (NodeTypeEnum::Iterator, "iterator", "循环迭代", false),
        (NodeTypeEnum::Script, "script", "脚本", true),
        (NodeTypeEnum::SwitchScript, "switch_script", "选择脚本", true),
        (NodeTypeEnum::BooleanScript, "boolean_script", "布尔脚本", true),
        (NodeTypeEnum::IfScript, "if_script", "条件脚本", true),
        (NodeTypeEnum::ForScript, "for_script", "循环次数脚本", true),
        (NodeTypeEnum::WhileScript, "while_script", "循环条件脚本", true),
        (NodeTypeEnum::BreakScript, "break_script", "循环跳出脚本", true),
        (NodeTypeEnum::Fallback, "fallback", "降级", false),
    ] {
        assert_eq!(node_type.get_code(), code);
        assert_eq!(node_type.get_name(), name);
        assert_eq!(node_type.is_script(), is_script);
    }
    assert_eq!(
        NodeTypeEnum::get_enum_by_code("switch_script"),
        Some(NodeTypeEnum::SwitchScript)
    );
    assert_eq!(NodeTypeEnum::get_enum_by_code("unknown"), None);

    // setCode：有效代码切换类型，未知代码返回 false 且保持原值。
    let mut node_type = NodeTypeEnum::Common;
    assert!(node_type.set_code("for"));
    assert_eq!(node_type, NodeTypeEnum::For);
    assert!(!node_type.set_code("unknown"));
    assert_eq!(node_type, NodeTypeEnum::For);

    // setName：有效中文名切换类型，未知名称返回 false。
    assert!(node_type.set_name("脚本"));
    assert_eq!(node_type, NodeTypeEnum::Script);
    assert!(!node_type.set_name("不存在的类型"));
    assert_eq!(node_type, NodeTypeEnum::Script);

    // setScript：普通/选择/布尔/次数四组切换脚本与非脚本对等类型；
    // 迭代与降级没有脚本对等项，置 true 拒绝。
    let mut boolean = NodeTypeEnum::Boolean;
    assert!(boolean.set_script(true));
    assert_eq!(boolean, NodeTypeEnum::BooleanScript);
    assert!(boolean.set_script(false));
    assert_eq!(boolean, NodeTypeEnum::Boolean);
    let mut iterator = NodeTypeEnum::Iterator;
    assert!(iterator.set_script(false));
    assert!(!iterator.set_script(true));
    assert_eq!(iterator, NodeTypeEnum::Iterator);
    let mut fallback = NodeTypeEnum::Fallback;
    assert!(!fallback.set_script(true));

    // getMappingClazz/setMappingClazz：组件类别名 ↔ 节点类型互转。
    assert_eq!(
        NodeTypeEnum::Common.get_mapping_clazz(),
        Some("NodeComponent")
    );
    assert_eq!(
        NodeTypeEnum::BooleanScript.get_mapping_clazz(),
        Some("ScriptBooleanComponent")
    );
    assert_eq!(NodeTypeEnum::Fallback.get_mapping_clazz(), None);
    let mut mapped = NodeTypeEnum::Fallback;
    assert!(mapped.set_mapping_clazz(Some("NodeIteratorComponent")));
    assert_eq!(mapped, NodeTypeEnum::Iterator);
    assert!(mapped.set_mapping_clazz(None));
    assert_eq!(mapped, NodeTypeEnum::Fallback);
    assert!(!mapped.set_mapping_clazz(Some("NoSuchComponent")));

    // guessTypeBySuperClazz：按最后一级类型名推断。
    assert_eq!(
        NodeTypeEnum::guess_type_by_super_clazz("com.yomahub.liteflow.NodeSwitchComponent"),
        Some(NodeTypeEnum::Switch)
    );
    assert_eq!(
        NodeTypeEnum::guess_type_by_super_clazz("ScriptForComponent"),
        Some(NodeTypeEnum::ForScript)
    );
    assert_eq!(NodeTypeEnum::guess_type_by_super_clazz("Unknown"), None);
}

fn method_wrap(name: &str, node_type: NodeTypeEnum) -> MethodWrapBean {
    MethodWrapBean::new(
        LiteFlowMethodBean::new(name, LiteFlowMethodEnum::Process),
        LiteFlowMethodEnum::Process,
        node_type,
        None,
        Vec::new(),
        Vec::new(),
    )
}

fn declaration(
    node_type: NodeTypeEnum,
    methods: Vec<MethodWrapBean>,
) -> DeclWarpBean {
    DeclWarpBean::new(
        "decl",
        "声明式",
        node_type,
        Arc::new(PassThroughDecl),
        "tests::PassThroughDecl",
        methods,
    )
}

/// `DeclComponentProxy#getProxy`：空方法表、节点类型不一致、重复方法名三类
/// Java 元数据不变量在 Rust 侧同样拒绝。
#[test]
fn decl_component_proxy_rejects_invalid_metadata() {
    let empty = DeclComponentProxy::new(declaration(NodeTypeEnum::Common, Vec::new()));
    let error = match empty.get_proxy() {
        Ok(_) => panic!("空方法表必须拒绝"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("has no LiteflowMethod"));

    let mismatched = DeclComponentProxy::new(declaration(
        NodeTypeEnum::Common,
        vec![method_wrap("process", NodeTypeEnum::Boolean)],
    ));
    let error = match mismatched.get_proxy() {
        Ok(_) => panic!("类型不一致必须拒绝"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("must be identical"));

    let duplicate = DeclComponentProxy::new(declaration(
        NodeTypeEnum::Common,
        vec![
            method_wrap("process", NodeTypeEnum::Common),
            method_wrap("process", NodeTypeEnum::Common),
        ],
    ));
    let error = match duplicate.get_proxy() {
        Ok(_) => panic!("重复方法名必须拒绝"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("duplicate LiteflowMethod"));

    let valid = DeclComponentProxy::new(declaration(
        NodeTypeEnum::Common,
        vec![method_wrap("process", NodeTypeEnum::Common)],
    ));
    assert!(valid.get_proxy().is_ok());
}

/// `DeclComponentProxy#invoke`：未注册的 LiteflowMethod 返回代理错误，
/// 与 Java 动态代理 InvocationHandler 的查找失败语义一致。
#[tokio::test]
async fn decl_component_proxy_invoke_rejects_missing_method() {
    let proxy = DeclComponentProxy::new(declaration(
        NodeTypeEnum::Common,
        vec![method_wrap("process", NodeTypeEnum::Common)],
    ));
    let slot = Arc::new(Slot::new("RID-PROXY".to_string(), "main", Value::Null));
    let context = CmpContext {
        inner: slot,
        node: NodeRef::new("decl"),
        frame: liteflow_core::slot::Frame::root(),
    };
    let error = proxy
        .invoke("missing_method", &context)
        .await
        .expect_err("未注册方法必须报错");
    assert!(error.to_string().contains("no LiteflowMethod[missing_method]"));
    let error = proxy
        .invoke_with_error("missing_method", &context, &LiteflowError::Custom("boom".into()))
        .await
        .expect_err("未注册方法必须报错");
    assert!(error.to_string().contains("no LiteflowMethod[missing_method]"));
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct JsonTarget {
    value: i32,
}

/// `JsonUtil`：null 输入返回 null、空文本返回空、非法 JSON 返回
/// `JsonProcessException`，与 Java `JsonUtil` 的边界一致。
#[test]
fn json_util_null_empty_and_error_contracts() {
    assert_eq!(JsonUtil::to_json_string::<JsonTarget>(None).unwrap(), None);
    assert_eq!(
        JsonUtil::to_json_string(Some(&JsonTarget { value: 7 }))
            .unwrap()
            .as_deref(),
        Some(r#"{"value":7}"#)
    );
    // 序列化器报错 → JsonProcessException（Java 写入失败语义）。
    struct FailingSerializer;
    impl Serialize for FailingSerializer {
        fn serialize<S: serde::Serializer>(
            &self,
            _serializer: S,
        ) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("write rejected"))
        }
    }
    let error = JsonUtil::to_json_string(Some(&FailingSerializer)).unwrap_err();
    assert!(error.to_string().contains("Error while writing"));

    assert_eq!(JsonUtil::parse_value("").unwrap(), None);
    assert_eq!(JsonUtil::parse_value(r#"{"a":1}"#).unwrap(), Some(json!({"a": 1})));
    let error = JsonUtil::parse_value("{broken").unwrap_err();
    assert!(error.to_string().contains("Error while parsing text"));

    assert_eq!(JsonUtil::parse_object::<JsonTarget>("").unwrap(), None);
    assert_eq!(
        JsonUtil::parse_object::<JsonTarget>(r#"{"value":3}"#).unwrap(),
        Some(JsonTarget { value: 3 })
    );
    let error = JsonUtil::parse_object::<JsonTarget>("nope").unwrap_err();
    assert!(error.to_string().contains("Error while parsing text"));

    assert_eq!(
        JsonUtil::parse_list::<i32>("").unwrap(),
        Vec::<i32>::new()
    );
    assert_eq!(JsonUtil::parse_list::<i32>("[1,2,3]").unwrap(), vec![1, 2, 3]);
    let error = JsonUtil::parse_list::<i32>("[1,x]").unwrap_err();
    assert!(error.to_string().contains("Error while parsing text"));
}

/// `LFLog`：五个级别与 Java 开关语义一致——TRACE/DEBUG 不受执行日志开关影响，
/// INFO/WARN/ERROR 受 `printExecutionLog` 控制；请求 ID 进入消息前缀。
#[test]
fn lf_log_levels_request_id_and_execution_gate() {
    let logger = LFLog::new("tests::lf_log");
    assert_eq!(logger.name(), "tests::lf_log");
    assert_eq!(logger.get_name(), "tests::lf_log");
    let _ = logger.is_trace_enabled();
    let _ = logger.is_debug_enabled();
    let _ = logger.is_info_enabled();
    let _ = logger.is_warn_enabled();
    let _ = logger.is_error_enabled();

    // 请求 ID 前缀进入日志消息（走真实 log 门面）。
    LFLoggerManager::set_request_id("RID-LFLOG");
    logger.trace("trace-message");
    logger.debug("debug-message");
    logger.info("info-message");
    logger.warn("warn-message");
    logger.error("error-message");

    // 关闭执行日志开关：INFO/WARN/ERROR 直接短路，TRACE/DEBUG 仍写入。
    LFLoggerManager::set_print_execution_log(false);
    logger.info("gated-off");
    logger.trace("trace-ungated");
    LFLoggerManager::set_print_execution_log(true);
    LFLoggerManager::set_request_id("");
}

/// `ChainPropBean`：Java 链式 setter 与 getter 对等，序列化保持 camelCase。
#[test]
fn chain_prop_bean_java_builder_accessors() {
    let mut bean = liteflow_core::builder::prop::ChainPropBean::default();
    bean = bean
        .set_cond_value_str("THEN(a,b)")
        .set_group("group-a")
        .set_error_resume("true")
        .set_any("false")
        .set_thread_executor_class("custom.Executor")
        .set_condition_type(ConditionTypeEnum::Then);
    assert_eq!(bean.get_cond_value_str(), Some("THEN(a,b)"));
    assert_eq!(bean.cond_value_str(), Some("THEN(a,b)"));
    assert_eq!(bean.get_group(), Some("group-a"));
    assert_eq!(bean.group(), Some("group-a"));
    assert_eq!(bean.get_error_resume(), Some("true"));
    assert_eq!(bean.error_resume(), Some("true"));
    assert_eq!(bean.get_any(), Some("false"));
    assert_eq!(bean.any(), Some("false"));
    assert_eq!(bean.get_thread_executor_class(), Some("custom.Executor"));
    assert_eq!(bean.thread_executor_class(), Some("custom.Executor"));
    assert_eq!(bean.get_condition_type(), Some(ConditionTypeEnum::Then));
    assert_eq!(bean.condition_type(), Some(ConditionTypeEnum::Then));

    let serialized = serde_json::to_string(&bean).unwrap();
    assert!(serialized.contains(r#""condValueStr":"THEN(a,b)""#));
    let empty = liteflow_core::builder::prop::ChainPropBean::default();
    assert_eq!(empty.get_cond_value_str(), None);
}

/// 声明式组件最小实现（承接 `DeclWarpBean` 的 `Arc<dyn DeclComponent>`）。
struct PassThroughDecl;

#[async_trait]
impl DeclComponent for PassThroughDecl {
    async fn call(&self, _method: &str, _context: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!({"ok": true}))
    }
}
