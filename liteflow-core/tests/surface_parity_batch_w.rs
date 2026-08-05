//! Java 表面语义回归批次 W：SPI 本地实现、实例编号、执行器与默认方法。
//!
//! 覆盖对象与 Java 对应关系：
//! - `LocalContextAware` 无容器空实现（Java 非 Spring 环境）
//! - `LiteFlowProxyUtil`（isDeclareCmp/proxy2NodeComponent/registerDeclWrapBean/
//!   isCglibProxyClass/getUserClass）
//! - `LocalDeclComponentParser` 元数据校验（Java `parseDeclBean`）
//! - `PathContentParserHolder` 注册与清理（Java Holder 语义）
//! - `DefaultNodeInstanceIdManageSpiImpl` 两行文件格式（Java 持久化协议）
//! - `ExecutorService` 关闭语义（Java `shutdown` 后拒绝新任务）
//! - `Executable` 默认成员（Java 接口默认方法）
//! - `BindWrapperCondition` 属性与内部回退（Java `ChainBindWrapperCondition`）

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use liteflow_core::core::proxy::{DeclWarpBean, LiteFlowProxyUtil, MethodWrapBean};
use liteflow_core::core::DeclComponent;
use liteflow_core::enums::{ExecuteableTypeEnum, LiteFlowMethodEnum, NodeTypeEnum};
use liteflow_core::exception::LFResult;
use liteflow_core::flow::element::condition::bind_wrapper_condition::BindWrapperCondition;
use liteflow_core::flow::element::Executable;
use liteflow_core::flow::entity::InstanceInfoDto;
use liteflow_core::flow::instance_id::{
    DefaultNodeInstanceIdManageSpiImpl, NodeInstanceIdManageSpi,
};
use liteflow_core::spi::holder::PathContentParserHolder;
use liteflow_core::spi::local::{LocalContextAware, LocalDeclComponentParser, LocalPathContentParser};
use liteflow_core::spi::{ContextAware, DeclComponentParser};
use liteflow_core::thread::ExecutorService;
use liteflow_core::{CmpContext, LiteflowError, NodeRef, Slot};
use serde_json::{Value, json};

fn method_wrap(name: &str, node_type: NodeTypeEnum) -> MethodWrapBean {
    MethodWrapBean::new(
        liteflow_core::core::proxy::LiteFlowMethodBean::new(name, LiteFlowMethodEnum::Process),
        LiteFlowMethodEnum::Process,
        node_type,
        None,
        Vec::new(),
        Vec::new(),
    )
}

fn declaration(methods: Vec<MethodWrapBean>) -> DeclWarpBean {
    DeclWarpBean::new(
        "decl",
        "声明式",
        NodeTypeEnum::Common,
        Arc::new(PassThroughDecl),
        "tests::PassThroughDecl",
        methods,
    )
}

/// `LocalContextAware`：无容器实现的查询恒空、注册原样返回、类型查询为 null，
/// 与 Java 非 Spring 环境的空实现语义一致。
#[test]
fn local_context_aware_is_a_stateless_empty_container() {
    let aware = LocalContextAware::new();
    assert!(aware.get_bean("missing").is_none());
    let bean: liteflow_core::spi::Bean = Arc::new(7_u32);
    assert!(Arc::ptr_eq(&aware.register_bean("key", Arc::clone(&bean)), &bean));
    let built: liteflow_core::spi::Bean =
        aware.register_or_get("factory", &|| Arc::new(String::from("new")));
    assert_eq!(built.downcast_ref::<String>().map(String::as_str), Some("new"));
    assert!(aware.get_beans_of_type(None).is_none());
    assert!(aware.get_beans_of_type(Some("any")).is_none());
    assert!(!aware.has_bean("missing"));
    assert!(!aware.has_bean_type("any"));
    let declaration = declaration(vec![method_wrap("process", NodeTypeEnum::Common)]);
    assert!(aware.register_decl_wrap_bean("decl", declaration).is_none());

    // trait 委托路径与 inherent 方法等价。
    let trait_object: &dyn ContextAware = &aware;
    assert!(trait_object.get_bean("missing").is_none());
    assert!(Arc::ptr_eq(
        &trait_object.register_bean("key", Arc::clone(&bean)),
        &bean
    ));
    assert!(!trait_object.has_bean("missing"));
    assert!(!trait_object.has_bean_type("any"));
}

/// `LiteFlowProxyUtil`：声明式元数据识别、代理生成/注册与 CGLIB 名称兼容。
#[tokio::test]
async fn lite_flow_proxy_util_decl_metadata_and_proxy_names() {
    let with_method = declaration(vec![method_wrap("process", NodeTypeEnum::Common)]);
    assert!(LiteFlowProxyUtil::is_declare_cmp(&with_method));
    assert!(!LiteFlowProxyUtil::is_declare_cmp(&declaration(Vec::new())));

    let proxy = LiteFlowProxyUtil::proxy2_node_component(with_method.clone()).unwrap();
    let alias = LiteFlowProxyUtil::proxy_to_decl_component(with_method.clone()).unwrap();
    // 两个入口各自生成独立代理，但行为等价：同一方法名走真实静态分派表。
    let slot = Arc::new(Slot::new("RID-PROXY".to_string(), "main", Value::Null));
    let context = CmpContext {
        inner: slot,
        node: NodeRef::new("decl"),
        frame: liteflow_core::slot::Frame::root(),
    };
    assert_eq!(proxy.call("process", &context).await.unwrap(), json!({"ok": true}));
    assert_eq!(alias.call("process", &context).await.unwrap(), json!({"ok": true}));

    let bus = liteflow_core::FlowBus::new();
    bus.try_register_decl_warp(with_method.clone()).unwrap();
    let error = match bus.try_register_decl_warp(declaration(Vec::new())) {
        Ok(()) => panic!("缺少声明式方法必须拒绝注册"),
        Err(error) => error,
    };
    assert!(error
        .to_string()
        .contains("does not contain liteflow declaration"));

    let error = match LiteFlowProxyUtil::proxy2_node_component(declaration(Vec::new())) {
        Ok(_) => panic!("空方法表必须拒绝"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("has no LiteflowMethod"));

    assert!(LiteFlowProxyUtil::is_cglib_proxy_class("com.example.Node$$Enhancer"));
    assert!(!LiteFlowProxyUtil::is_cglib_proxy_class("com.example.Node"));
    assert_eq!(
        LiteFlowProxyUtil::get_user_class("com.example.Node$$Enhancer"),
        "com.example.Node"
    );
    assert_eq!(
        LiteFlowProxyUtil::get_user_class("com.example.Node"),
        "com.example.Node"
    );
}

/// `LocalDeclComponentParser`：缺少声明式方法时按 Java `NotSupportDecl` 拒绝，
/// 元数据齐全时透传；优先级为 2。
#[test]
fn local_decl_component_parser_validates_metadata() {
    let parser = LocalDeclComponentParser::new();
    assert_eq!(parser.priority(), 2);

    let parsed = parser
        .parse_decl_bean(declaration(vec![method_wrap("process", NodeTypeEnum::Common)]))
        .unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].get_node_id(), "decl");

    let error = match parser.parse_decl_bean(declaration(Vec::new())) {
        Ok(_) => panic!("缺少声明式方法必须拒绝"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("does not contain liteflow declaration"));

    // trait 委托路径。
    let trait_object: &dyn DeclComponentParser = &parser;
    assert!(trait_object
        .parse_decl_bean(declaration(vec![method_wrap("process", NodeTypeEnum::Common)]))
        .is_ok());
}

/// `PathContentParserHolder`：显式注册覆盖默认实现，`clean` 恢复默认。
#[test]
fn path_content_parser_holder_register_and_clean() {
    PathContentParserHolder::clean();
    let default = PathContentParserHolder::load_context_aware();
    let same = PathContentParserHolder::load_path_content_parser();
    assert!(Arc::ptr_eq(&default, &same));

    let custom: Arc<dyn liteflow_core::spi::PathContentParser> =
        Arc::new(LocalPathContentParser::new());
    PathContentParserHolder::register(Arc::clone(&custom));
    assert!(Arc::ptr_eq(
        &PathContentParserHolder::load_context_aware(),
        &custom
    ));
    PathContentParserHolder::clean();
    assert!(!Arc::ptr_eq(&PathContentParserHolder::load_context_aware(), &custom));
}

/// `DefaultNodeInstanceIdManageSpiImpl`：Java 两行文件格式（el_md5 + JSON 列表）
/// 的写入与回读；空白 chain、缺失文件与空列表按 Java 返回空。
#[test]
fn node_instance_id_spi_persists_java_two_line_format() {
    let base = std::env::temp_dir().join(format!("liteflow-instance-id-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let spi = DefaultNodeInstanceIdManageSpiImpl::with_base_path(base.clone());
    assert_eq!(spi.base_path(), PathBuf::from(&base));

    assert!(spi.read_instance_id_file("").unwrap().is_empty());
    assert!(spi.read_instance_id_file("missing_chain").unwrap().is_empty());
    spi.write_instance_id_file(&[], "md5-a", "chain-a").unwrap();

    let infos = vec![
        InstanceInfoDto::new("chain-a", "a", "a_1", 0),
        InstanceInfoDto::new("chain-a", "b", "b_1", 1),
    ];
    spi.write_instance_id_file(&infos, "md5-a", "chain-a").unwrap();
    let lines = spi.read_instance_id_file("chain-a").unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "md5-a");
    assert!(lines[1].contains("a_1"));

    // 实例编号生成：`nodeId_shortUuid_index` 格式。
    let spi_for_gen = DefaultNodeInstanceIdManageSpiImpl::with_base_path(base.clone());
    let generated = NodeInstanceIdManageSpi::gen_instance_id(&spi_for_gen, "chain-a", "node-a", 3);
    assert!(generated.starts_with("node-a_") && generated.ends_with("_3"));

    let _ = std::fs::remove_dir_all(&base);
}

/// `ExecutorService`：Java 线程池构造参数语义（core/maximum 收敛、并发上限），
/// `shutdown` 后拒绝新任务。
#[tokio::test]
async fn executor_service_executes_and_rejects_after_shutdown() {
    let service = ExecutorService::new(1, 2, 4, "tests-pool");
    assert_eq!(service.core_pool_size(), 1);
    assert_eq!(service.maximum_pool_size(), 2);
    assert_eq!(service.queue_capacity(), 4);
    assert_eq!(service.thread_name(), "tests-pool");
    assert!(!service.is_shutdown());

    let result = service.execute(async { 40 + 2 }).await.unwrap();
    assert_eq!(result, 42);
    assert!(service.active_count() >= 0);

    service.shutdown();
    assert!(service.is_shutdown());
    let error = service
        .execute(async { 1 })
        .await
        .expect_err("关闭后必须拒绝新任务");
    assert!(error.to_string().contains("has been shut down"));
    assert!(service.await_termination(Duration::from_secs(1)).await);

    // 构造参数收敛：core 至少 1、core ≤ maximum。
    let clamped = ExecutorService::new(0, 0, 2, "clamped-pool");
    assert_eq!(clamped.core_pool_size(), 1);
    assert_eq!(clamped.maximum_pool_size(), 1);
}

/// `Executable` 默认成员：Node 类型默认收集自身 ID、其余类型返回空；
/// tag/PRE/FINALLY/applyCmpData/setAccessResult 默认行为与 Java 接口一致。
#[tokio::test]
async fn executable_default_members_follow_java_contracts() {
    struct Probe {
        id: String,
        execute_type: ExecuteableTypeEnum,
    }
    #[async_trait]
    impl Executable for Probe {
        async fn execute(&self, _ctx: &liteflow_core::slot::Ctx, _frame: &liteflow_core::Frame) -> LFResult<Value> {
            Ok(Value::Null)
        }
        fn execute_type(&self) -> ExecuteableTypeEnum {
            self.execute_type
        }
        fn id(&self) -> &str {
            &self.id
        }
    }

    let node = Probe {
        id: "probe".to_string(),
        execute_type: ExecuteableTypeEnum::Node,
    };
    assert_eq!(node.collect_node_ids(), vec!["probe".to_string()]);
    let condition = Probe {
        id: "probe".to_string(),
        execute_type: ExecuteableTypeEnum::Condition,
    };
    assert!(condition.collect_node_ids().is_empty());
    assert_eq!(condition.tag(), None);
    assert!(!condition.is_pre_or_finally());

    let slot = Arc::new(Slot::new("RID-EXE".to_string(), "main", Value::Null));
    let frame = liteflow_core::Frame::root();
    let ctx = liteflow_core::slot::Ctx::new(slot);
    condition.apply_chain_cmp_data("data");
    condition.set_access_result(&frame, false);
    assert!(condition.is_access(&ctx, &frame).await);
}

/// `BindWrapperCondition`：Java `ChainBindWrapperCondition` 的属性携带与
/// 内部对象回退（id/tag 未显式设置时取内部可执行对象）。
#[tokio::test]
async fn bind_wrapper_condition_properties_and_inner_fallbacks() {
    let inner: Arc<dyn Executable> = Arc::new(InnerProbe);
    let plain = BindWrapperCondition::new(Arc::clone(&inner), vec![("k".to_string(), "v".to_string())]);
    assert_eq!(plain.id(), "inner-probe");
    assert_eq!(plain.tag(), Some("inner-tag"));
    assert_eq!(plain.thread_pool(), None);
    assert!(!plain.is_pre_or_finally());
    let slot = Arc::new(Slot::new("RID-BIND".to_string(), "main", Value::Null));
    let frame = liteflow_core::Frame::root();
    let ctx = liteflow_core::slot::Ctx::new(slot);
    let result = plain.execute(&ctx, &frame).await.unwrap();
    assert_eq!(result, json!({"bound": true}));

    let with_properties = BindWrapperCondition::with_properties(
        Arc::clone(&inner),
        vec![("k".to_string(), "v".to_string())],
        Some("outer".to_string()),
        Some("outer-tag".to_string()),
        Some("custom.Executor".to_string()),
    );
    assert_eq!(with_properties.id(), "outer");
    assert_eq!(with_properties.tag(), Some("outer-tag"));
    assert_eq!(with_properties.thread_pool(), Some("custom.Executor"));
}

/// 内部可执行探针：id/tag 固定值，execute 返回绑定结果。
struct InnerProbe;

#[async_trait]
impl Executable for InnerProbe {
    async fn execute(&self, _ctx: &liteflow_core::slot::Ctx, _frame: &liteflow_core::Frame) -> LFResult<Value> {
        Ok(json!({"bound": true}))
    }

    fn id(&self) -> &str {
        "inner-probe"
    }

    fn tag(&self) -> Option<&str> {
        Some("inner-tag")
    }
}

/// 声明式组件最小实现（承接 `DeclWarpBean` 的 `Arc<dyn DeclComponent>`）。
struct PassThroughDecl;

#[async_trait]
impl DeclComponent for PassThroughDecl {
    async fn call(&self, _method: &str, _context: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!({"ok": true}))
    }
}
