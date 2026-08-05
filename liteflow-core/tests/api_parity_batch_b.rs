//! 未触达公开 API 的 Java v2.16.0 对等补测（批次 B）。
//!
//! 覆盖现有测试未调用的入口，均对照 Java 原版语义：
//! - `CmpStep#buildString/buildStringWithInstanceId`
//! - `LiteflowResponse#getRollbackStepQueue/getRollbackStepStrWithoutTime`
//! - `SerialsUtil#genSerialNo/nextSerial/randomNum12/randomNum8/genToken`
//! - `LiteFlowException` 的 code/cause 构造变体
//! - `MonitorBus#withQueueLimit/record/queueLimit` 与 `StatEntry#avgTimeMs`、
//!   `CompStatistics` 别名
//! - `ScriptKind#fromCode/checkReturn`
//! - `BindWrapperCondition#withProperties`、`MethodWrapBean` 元数据访问器、
//!   `DefaultNodeInstanceIdManageSpiImpl#basePath`

use liteflow_core::core::proxy::{LiteFlowMethodBean, MethodWrapBean, ParameterWrapBean};
use liteflow_core::enums::{LiteFlowMethodEnum, NodeTypeEnum};
use liteflow_core::exception::{LFResult, LiteFlowException};
use liteflow_core::flow::element::condition::bind_wrapper_condition::BindWrapperCondition;
use liteflow_core::flow::element::executable::Executable;
use liteflow_core::flow::instance_id::DefaultNodeInstanceIdManageSpiImpl;
use liteflow_core::monitor::{CompStatistics, MonitorBus};
use liteflow_core::script::ScriptKind;
use liteflow_core::util::SerialsUtil;
use liteflow_core::{CmpStep, CmpStepTypeEnum, LiteflowResponse, Slot};
use serde_json::{Value, json};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

/// CmpStep 基础文本与实例编号文本。
///
/// 对应 Java: `CmpStep#buildString`（nodeName 空白→仅 ID）与
/// `CmpStep#buildStringWithInstanceId`（`id[instanceId]`）。
#[test]
fn cmp_step_build_strings_match_java_formats() {
    let blank = CmpStep::new("a", "", CmpStepTypeEnum::Single);
    assert_eq!(blank.build_string(), "a");
    let named = CmpStep::new("b", "节点B", CmpStepTypeEnum::Single);
    assert_eq!(named.build_string(), "b[节点B]");

    let mut with_instance = CmpStep::new("c", "节点C", CmpStepTypeEnum::Single);
    with_instance.set_node_instance_id("c_7");
    assert_eq!(with_instance.build_string_with_instance_id(), "c[c_7]");
    // 无实例编号时按 Java 语义输出空后缀
    let no_instance = CmpStep::new("d", "节点D", CmpStepTypeEnum::Single);
    assert_eq!(no_instance.build_string_with_instance_id(), "d[]");
}

/// LiteflowResponse 回滚步骤队列与无耗时文本。
///
/// 对应 Java: `LiteflowResponse#getRollbackStepQueue` 与
/// `getRollbackStepStrWithoutTime`。
#[test]
fn response_rollback_aliases_match_java() {
    let slot = Slot::new("RID-RB".to_string(), "main", json!(null));
    let mut rb = CmpStep::new("r", "回滚节点", CmpStepTypeEnum::Single);
    rb.finish_rollback(true, None);
    rb.set_rollback_time_spent(30);
    slot.add_rollback_step(rb);
    let response = LiteflowResponse::new_main_response(Arc::new(slot));

    assert_eq!(response.get_rollback_step_queue().len(), 1);
    assert_eq!(response.get_rollback_step_queue()[0].get_node_id(), "r");
    assert_eq!(response.get_rollback_step_str_without_time(), "r[回滚节点]");
    assert_eq!(response.rollback_step_str(), "r[回滚节点]");
}

/// SerialsUtil 序列号/随机数/令牌工具与 Java 形状一致。
///
/// 对应 Java: `SerialsUtil#genSerialNo/nextSerial/randomNum12/randomNum8/genToken`。
#[test]
fn serials_util_generators_follow_java_shapes() {
    let serial = SerialsUtil::gen_serial_no();
    // 14 位时间 + 3 位随机 + 3 位序号
    assert_eq!(serial.len(), 20);
    assert!(serial.chars().all(|c| c.is_ascii_digit()));

    let next = SerialsUtil::next_serial();
    assert_eq!(next.len(), 3);
    assert!(next.chars().all(|c| c.is_ascii_digit()));

    // 12/8 位数字，不足位前导补零
    let n12 = SerialsUtil::random_num12(42);
    assert_eq!(n12.len(), 12);
    assert!(n12.chars().all(|c| c.is_ascii_digit()));
    let n8 = SerialsUtil::random_num8(42);
    assert_eq!(n8.len(), 8);
    assert!(n8.chars().all(|c| c.is_ascii_digit()));

    // Java genToken = 8 位 32 进制 + 8 位文件 UUID
    let token = SerialsUtil::gen_token();
    assert_eq!(token.len(), 16);
    assert!(token.chars().all(|c| c.is_ascii_alphanumeric()));
}

/// LiteFlowException 的 code/cause 构造变体。
///
/// 对应 Java: `LiteFlowException(String,String)`、`LiteFlowException(Throwable)`、
/// `LiteFlowException(String,Throwable)`。
#[test]
fn lite_flow_exception_variants_preserve_code_and_cause() {
    #[derive(Debug)]
    struct Boom;
    impl std::fmt::Display for Boom {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "boom")
        }
    }
    impl Error for Boom {}

    let with_code = LiteFlowException::with_code("E-001", "业务失败");
    assert_eq!(with_code.get_code(), Some("E-001"));
    assert_eq!(with_code.get_message(), "业务失败");

    let from_cause = LiteFlowException::from_cause(Boom);
    assert_eq!(from_cause.get_message(), "boom");
    assert!(from_cause.get_cause().is_some());

    let with_cause = LiteFlowException::with_cause("外层失败", Boom);
    assert_eq!(with_cause.get_message(), "外层失败");
    assert!(with_cause.get_cause().is_some());
}

/// MonitorBus 队列容量工厂、记录与统计聚合。
///
/// 对应 Java: `MonitorBus#MonitorBus(int queueLimit)`、`record` 统计路径与
/// `StatEntry#avgTimeMs`。
#[test]
fn monitor_bus_records_and_stat_entry_aggregate() {
    let bus = MonitorBus::with_queue_limit(4);
    assert_eq!(bus.queue_limit(), 4);

    bus.record("node-a", Duration::from_millis(100), true);
    bus.record("node-a", Duration::from_millis(300), true);
    bus.record("node-a", Duration::from_millis(200), false);

    let report = bus.report();
    let entry = report
        .iter()
        .find(|stat| stat.component_clazz_name() == "node-a")
        .expect("node-a 应有统计项");
    // Java MonitorBus 报表的平均耗时 = 总耗时/总次数
    assert_eq!(entry.get_time_spent(), 200);
}

/// CompStatistics 的 Rust 别名与 Java 命名访问器共享状态。
#[test]
fn comp_statistics_aliases_share_state() {
    let mut stats = CompStatistics::new("com.example.CmpA", 500);
    stats.set_memory_spent(1024);

    assert_eq!(stats.component_clazz_name(), "com.example.CmpA");
    assert_eq!(stats.get_component_clazz_name(), "com.example.CmpA");
    assert_eq!(stats.time_spent(), 500);
    assert_eq!(stats.get_time_spent(), 500);
    assert_eq!(stats.memory_spent(), 1024);
    assert_eq!(stats.get_memory_spent(), 1024);
    // recordTime 在构造时记录真实时间戳（毫秒），与 Java new 语义一致
    assert!(stats.record_time() > 0);
    assert!(stats.get_record_time() > 0);
}

/// ScriptKind 代码映射与返回类型校验。
///
/// 对应 Java: `NodeTypeEnum` 脚本类型到五类节点的转换与各脚本组件返回约束。
#[test]
fn script_kind_code_mapping_and_return_checks() {
    assert_eq!(ScriptKind::from_code("script"), Some(ScriptKind::Common));
    assert_eq!(
        ScriptKind::from_code("boolean_script"),
        Some(ScriptKind::Boolean)
    );
    assert_eq!(
        ScriptKind::from_code("switch_script"),
        Some(ScriptKind::Switch)
    );
    assert_eq!(ScriptKind::from_code("for_script"), Some(ScriptKind::For));
    assert_eq!(
        ScriptKind::from_code("iterator_script"),
        Some(ScriptKind::Iterator)
    );
    assert_eq!(ScriptKind::from_code("unknown"), None);

    assert!(
        ScriptKind::Common
            .check_return("n1", json!("anything"))
            .is_ok()
    );
    assert!(ScriptKind::Boolean.check_return("n1", json!(true)).is_ok());
    assert!(
        ScriptKind::Boolean
            .check_return("n1", json!("not-bool"))
            .is_err()
    );
    assert!(
        ScriptKind::Switch
            .check_return("n1", json!("target"))
            .is_ok()
    );
    assert!(ScriptKind::Switch.check_return("n1", Value::Null).is_ok());
    assert!(ScriptKind::For.check_return("n1", json!(3)).is_ok());
    assert!(ScriptKind::For.check_return("n1", json!("three")).is_err());
    assert!(
        ScriptKind::Iterator
            .check_return("n1", json!([1, 2]))
            .is_ok()
    );
    assert!(ScriptKind::Iterator.check_return("n1", json!(1)).is_err());
}

/// BindWrapperCondition 属性包装：ID/tag/线程池进入真实对象。
///
/// 对应 Java: Condition 的 id/tag/threadPool 属性操作在绑定包装上的落位。
#[tokio::test]
async fn bind_wrapper_condition_with_properties_carries_metadata() {
    let captured = Arc::new(std::sync::Mutex::new(None::<String>));
    struct BindProbe {
        captured: Arc<std::sync::Mutex<Option<String>>>,
    }
    #[async_trait::async_trait]
    impl Executable for BindProbe {
        async fn execute(
            &self,
            _ctx: &liteflow_core::slot::Ctx,
            frame: &liteflow_core::Frame,
        ) -> LFResult<Value> {
            *self.captured.lock().unwrap() = frame.find_bind("tenant").map(ToOwned::to_owned);
            Ok(Value::Null)
        }
        fn id(&self) -> &str {
            "probe"
        }
    }
    let inner: Arc<dyn Executable> = Arc::new(BindProbe {
        captured: Arc::clone(&captured),
    });
    let wrapper = BindWrapperCondition::with_properties(
        inner,
        vec![("tenant".to_string(), "acme".to_string())],
        Some("wrapper-id".to_string()),
        Some("wrapper-tag".to_string()),
        Some("com.example.Pool".to_string()),
    );
    assert_eq!(wrapper.id(), "wrapper-id");
    assert_eq!(wrapper.tag(), Some("wrapper-tag"));
    assert_eq!(wrapper.thread_pool(), Some("com.example.Pool"));
    let ctx = liteflow_core::Ctx::new(Arc::new(liteflow_core::Slot::new(
        "r".to_string(),
        "c",
        Value::Null,
    )));
    let _ = wrapper.execute(&ctx, &liteflow_core::Frame::root()).await;
    assert_eq!(*captured.lock().unwrap(), Some("acme".to_string()));
}

/// MethodWrapBean 元数据访问器与重试范围修改。
#[test]
fn method_wrap_bean_metadata_accessors_round_trip() {
    let parameters = vec![ParameterWrapBean::new("u64", Some("count"), 0)];
    let mut method = MethodWrapBean::new(
        LiteFlowMethodBean::new("sum", LiteFlowMethodEnum::Process),
        LiteFlowMethodEnum::Process,
        NodeTypeEnum::Common,
        Some(3),
        vec!["java.lang.Exception".to_string()],
        parameters,
    );

    assert_eq!(method.liteflow_method(), LiteFlowMethodEnum::Process);
    assert_eq!(method.liteflow_retry(), Some(3));
    assert_eq!(method.parameter_wrap_bean_list().len(), 1);

    method.set_retry_for(vec!["java.io.IOException".to_string()]);
    let retry_for = method.retry_for();
    assert_eq!(retry_for.len(), 1);
    assert_eq!(retry_for[0], "java.io.IOException");
}

/// 默认实例编号实现的目录与文件生命周期。
#[test]
fn default_instance_id_spi_exposes_base_path() {
    let dir = std::env::temp_dir().join("liteflow-node-instance-id-test");
    let spi = DefaultNodeInstanceIdManageSpiImpl::with_base_path(&dir);
    assert_eq!(spi.base_path(), dir.as_path());
    // 文件写入/读取按 Java 两行格式往返
    let chain_id = "instance_chain";
    let dto = liteflow_core::flow::entity::InstanceInfoDto::new("instance_chain", "a", "a_1", 0);
    spi.write_instance_id_file(&[dto], "md5-1", chain_id)
        .expect("写入实例编号文件");
    // Java 两行格式：第一行 el_md5，第二行 instance_id_list 的 serde JSON
    let loaded = spi
        .read_instance_id_file(chain_id)
        .expect("读取实例编号文件");
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0], "md5-1");
    assert!(loaded[1].contains("\"nodeId\":\"a\""));
    assert!(loaded[1].contains("\"instanceId\":\"a_1\""));
    let _ = std::fs::remove_dir_all(&dir);
}
