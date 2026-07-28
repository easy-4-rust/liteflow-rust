//! 默认 Chain 构建流程与节点实例编号 SPI 的真实接线验收。

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use liteflow_core::flow::instance_id::NodeInstanceIdManageSpi;
use liteflow_core::{
    FlowBus, InstanceInfoDto, LFResult, LiteflowConfig, LiteflowConfigGetter, cmp, parse_el, rule,
};
use serde_json::Value;

/// 测试用持久化 SPI：保存 Java 两行格式，并记录真实读写与生成次数。
struct PersistedInstanceIdSpi {
    prefix: &'static str,
    files: Arc<Mutex<HashMap<String, Vec<String>>>>,
    reads: Arc<AtomicUsize>,
    writes: Arc<AtomicUsize>,
    generations: Arc<AtomicUsize>,
}

impl NodeInstanceIdManageSpi for PersistedInstanceIdSpi {
    fn gen_instance_id(&self, _chain_id: &str, node_id: &str, occurrence: usize) -> String {
        self.generations.fetch_add(1, Ordering::SeqCst);
        format!("{node_id}-{}-{occurrence}", self.prefix)
    }

    fn read_instance_id_file(&self, chain_id: &str) -> LFResult<Vec<String>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .files
            .lock()
            .expect("实例编号测试存储锁不应中毒")
            .get(chain_id)
            .cloned()
            .unwrap_or_default())
    }

    fn write_instance_id_file(
        &self,
        instance_id_list: &[InstanceInfoDto],
        el_md5: &str,
        chain_id: &str,
    ) -> LFResult<()> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let json = serde_json::to_string(instance_id_list)
            .expect("完整 InstanceInfoDto 应可序列化为 Java camelCase JSON");
        self.files
            .lock()
            .expect("实例编号测试存储锁不应中毒")
            .insert(chain_id.to_string(), vec![el_md5.to_string(), json]);
        Ok(())
    }
}

fn register_null_component(bus: &FlowBus) {
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
}

/// 验证启用时写入和跨 FlowBus 恢复稳定编号，禁用时完全不触碰 SPI。
///
/// 对应 Java:
/// `LiteFlowChainELBuilder#compileChain`、
/// `BaseNodeInstanceIdManageSpi#setNodesInstanceId`。
#[tokio::test]
async fn default_chain_build_persists_restores_and_gates_node_instance_ids() {
    LiteflowConfigGetter::clean();
    let mut config = LiteflowConfig::default();
    config.set_enable_node_instance_id(true);
    LiteflowConfigGetter::set_liteflow_config(config);

    let files = Arc::new(Mutex::new(HashMap::new()));
    let reads = Arc::new(AtomicUsize::new(0));
    let writes = Arc::new(AtomicUsize::new(0));
    let generations = Arc::new(AtomicUsize::new(0));

    let first_bus = FlowBus::new();
    first_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "first",
        files: files.clone(),
        reads: reads.clone(),
        writes: writes.clone(),
        generations: generations.clone(),
    }));
    register_null_component(&first_bus);
    first_bus
        .add_chain("stable-chain", "THEN(a, a)")
        .expect("首次构建应生成并写入实例编号");

    assert_eq!(reads.load(Ordering::SeqCst), 1);
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    assert_eq!(generations.load(Ordering::SeqCst), 2);
    let first_response = first_bus.execute("stable-chain").await;
    let first_ids = first_response
        .steps
        .iter()
        .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        first_ids,
        vec![Some("a-first-0".to_string()), Some("a-first-1".to_string())]
    );

    // 使用全新的 FlowBus 和会生成不同前缀的 SPI，证明编号来自持久化快照而非进程缓存。
    let restored_bus = FlowBus::new();
    restored_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "restored",
        files: files.clone(),
        reads: reads.clone(),
        writes: writes.clone(),
        generations: generations.clone(),
    }));
    register_null_component(&restored_bus);
    restored_bus
        .add_chain("stable-chain", "THEN(a, a)")
        .expect("相同 EL 摘要应恢复持久化实例编号");

    assert_eq!(reads.load(Ordering::SeqCst), 2);
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    assert_eq!(generations.load(Ordering::SeqCst), 2);
    let restored_response = restored_bus.execute("stable-chain").await;
    let restored_ids = restored_response
        .steps
        .iter()
        .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(restored_ids, first_ids);

    // EL 变化时不能继续复用旧快照；热刷新必须重新生成并覆盖持久化内容。
    restored_bus
        .reload_chain("stable-chain", "THEN(a, a, a)")
        .expect("变化后的 EL 应重新生成实例编号");
    let reloaded_response = restored_bus.execute("stable-chain").await;
    let reloaded_ids = reloaded_response
        .steps
        .iter()
        .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        reloaded_ids,
        vec![
            Some("a-restored-0".to_string()),
            Some("a-restored-1".to_string()),
            Some("a-restored-2".to_string())
        ]
    );
    assert_eq!(reads.load(Ordering::SeqCst), 3);
    assert_eq!(writes.load(Ordering::SeqCst), 2);
    assert_eq!(generations.load(Ordering::SeqCst), 5);

    // XML/JSON/YAML 最终都汇入 ParserHelper；验证规则源路径也使用同一持久化主干。
    let parser_bus = FlowBus::new();
    parser_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "parser",
        files: files.clone(),
        reads: reads.clone(),
        writes: writes.clone(),
        generations: generations.clone(),
    }));
    register_null_component(&parser_bus);
    rule::load_xml_str(
        &parser_bus,
        r#"<flow><chain name="xml-chain">THEN(a, a)</chain></flow>"#,
    )
    .expect("XML ParserHelper 路径应生成并持久化实例编号");
    let parser_response = parser_bus.execute("xml-chain").await;
    let parser_ids = parser_response
        .steps
        .iter()
        .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        parser_ids,
        vec![
            Some("a-parser-0".to_string()),
            Some("a-parser-1".to_string())
        ]
    );
    assert_eq!(reads.load(Ordering::SeqCst), 4);
    assert_eq!(writes.load(Ordering::SeqCst), 3);
    assert_eq!(generations.load(Ordering::SeqCst), 7);

    // Java execute2RespWithEL 也通过 LiteFlowChainELBuilder 构建匿名 Chain；
    // 这里验证动态 EL 不是只登记 MD5，而是真实触发实例编号 SPI 并把编号带入步骤。
    let anonymous_bus = FlowBus::new();
    anonymous_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "anonymous",
        files: files.clone(),
        reads: reads.clone(),
        writes: writes.clone(),
        generations: generations.clone(),
    }));
    register_null_component(&anonymous_bus);
    let anonymous_response = anonymous_bus.execute_with_el("THEN(a, a)").await;
    assert!(
        anonymous_response.is_success(),
        "{}",
        anonymous_response.message
    );
    let anonymous_ids = anonymous_response
        .steps
        .iter()
        .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        anonymous_ids,
        vec![
            Some("a-anonymous-0".to_string()),
            Some("a-anonymous-1".to_string())
        ]
    );
    assert_eq!(reads.load(Ordering::SeqCst), 5);
    assert_eq!(writes.load(Ordering::SeqCst), 4);
    assert_eq!(generations.load(Ordering::SeqCst), 9);

    // 相同规范化 EL 应命中内存中的 elMd5 缓存，不得再次读取或写入持久化快照。
    let cached_anonymous_response = anonymous_bus.execute_with_el(" THEN( a, a ) ;;; ").await;
    assert!(cached_anonymous_response.is_success());
    assert_eq!(
        cached_anonymous_response
            .steps
            .iter()
            .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
            .collect::<Vec<_>>(),
        anonymous_ids
    );
    assert_eq!(reads.load(Ordering::SeqCst), 5);
    assert_eq!(writes.load(Ordering::SeqCst), 4);
    assert_eq!(generations.load(Ordering::SeqCst), 9);

    // Java 只给 route 的主体 Condition 编号；路由判断节点本身不进入持久化 DTO。
    let route_bus = FlowBus::new();
    route_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "route",
        files: files.clone(),
        reads: reads.clone(),
        writes: writes.clone(),
        generations: generations.clone(),
    }));
    route_bus.register("route_check", cmp(|_| async { Ok(Value::Bool(true)) }));
    register_null_component(&route_bus);
    route_bus
        .add_route_chain(
            "route-instance-chain",
            "instance",
            "route_check",
            "THEN(a, a)",
        )
        .expect("route Chain 的主体也应通过标准 Builder 生成实例编号");
    let route_responses = route_bus
        .execute_route_chain(Some("instance"), Value::Null)
        .await
        .expect("布尔路由应命中");
    assert_eq!(route_responses.len(), 1);
    assert_eq!(
        route_responses[0]
            .steps
            .iter()
            .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
            .collect::<Vec<_>>(),
        vec![Some("a-route-0".to_string()), Some("a-route-1".to_string())]
    );
    assert_eq!(reads.load(Ordering::SeqCst), 6);
    assert_eq!(writes.load(Ordering::SeqCst), 5);
    assert_eq!(generations.load(Ordering::SeqCst), 11);
    let route_snapshot = files
        .lock()
        .expect("实例编号测试存储锁不应中毒")
        .get("route-instance-chain")
        .cloned()
        .expect("route Chain 应写入实例编号快照");
    assert!(
        !route_snapshot[1].contains("route_check"),
        "路由判断节点不应进入主体实例编号 DTO"
    );

    // Rust 公开的类型化 AST 入口也必须进入统一 Builder，不能绕过实例编号主干。
    // 使用两个独立 FlowBus 证明摘要由 AST 的稳定编码产生，而非复用进程内对象。
    let parsed_el = parse_el("THEN(a, a)").expect("测试 EL 应可解析为类型化 AST");
    let parsed_bus = FlowBus::new();
    parsed_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "parsed",
        files: files.clone(),
        reads: reads.clone(),
        writes: writes.clone(),
        generations: generations.clone(),
    }));
    register_null_component(&parsed_bus);
    parsed_bus
        .add_chain_el("parsed-chain", parsed_el.clone())
        .expect("类型化 AST 应通过统一 Builder 生成实例编号");
    let parsed_response = parsed_bus.execute("parsed-chain").await;
    let parsed_ids = parsed_response
        .steps
        .iter()
        .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        parsed_ids,
        vec![
            Some("a-parsed-0".to_string()),
            Some("a-parsed-1".to_string())
        ]
    );
    assert_eq!(reads.load(Ordering::SeqCst), 7);
    assert_eq!(writes.load(Ordering::SeqCst), 6);
    assert_eq!(generations.load(Ordering::SeqCst), 13);

    let restored_parsed_bus = FlowBus::new();
    restored_parsed_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "restored-parsed",
        files: files.clone(),
        reads: reads.clone(),
        writes: writes.clone(),
        generations: generations.clone(),
    }));
    register_null_component(&restored_parsed_bus);
    restored_parsed_bus
        .add_chain_el("parsed-chain", parsed_el)
        .expect("相同类型化 AST 应恢复已有实例编号");
    let restored_parsed_response = restored_parsed_bus.execute("parsed-chain").await;
    assert_eq!(
        restored_parsed_response
            .steps
            .iter()
            .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
            .collect::<Vec<_>>(),
        parsed_ids
    );
    assert_eq!(reads.load(Ordering::SeqCst), 8);
    assert_eq!(writes.load(Ordering::SeqCst), 6);
    assert_eq!(generations.load(Ordering::SeqCst), 13);

    // Java 默认关闭 enableNodeInstanceId；禁用时不读取、不生成、不写入，也不伪造编号。
    LiteflowConfigGetter::clean();
    let disabled_reads = Arc::new(AtomicUsize::new(0));
    let disabled_writes = Arc::new(AtomicUsize::new(0));
    let disabled_generations = Arc::new(AtomicUsize::new(0));
    let disabled_bus = FlowBus::new();
    disabled_bus.set_instance_id_spi(Arc::new(PersistedInstanceIdSpi {
        prefix: "disabled",
        files,
        reads: disabled_reads.clone(),
        writes: disabled_writes.clone(),
        generations: disabled_generations.clone(),
    }));
    register_null_component(&disabled_bus);
    disabled_bus
        .add_chain("disabled-chain", "THEN(a)")
        .expect("禁用实例编号时仍应正常构建 Chain");
    let disabled_response = disabled_bus.execute("disabled-chain").await;

    assert_eq!(disabled_reads.load(Ordering::SeqCst), 0);
    assert_eq!(disabled_writes.load(Ordering::SeqCst), 0);
    assert_eq!(disabled_generations.load(Ordering::SeqCst), 0);
    assert_eq!(disabled_response.steps[0].get_node_instance_id(), None);
    LiteflowConfigGetter::clean();
}
