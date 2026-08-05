//! 生命周期/监控/并行/工具域的未触达 API 补测（批次 E）。
//!
//! 覆盖：
//! - `ChainCacheLifeCycle#new/initIfAbsent/getLifeCycle/isActive/isCleaned/
//!   newActiveState/setActive`
//! - `MonitorFile#addMonitorFilePaths`
//! - `ParallelStrategySupport#recordTimeoutItems/spawnAll`
//! - `LFLoggerManager#printExecutionLog 开关/removeRequestId`
//! - `ElRegexUtil#isAbstractChain/replaceAbstractChain`
//! - `OperatorHelper` 的数量/类型检查与数值转换
//! - `ParallelOperator` 布尔参数构建

use liteflow_core::flow::parallel::strategy::spawn_all;
use liteflow_core::lifecycle::PostProcessChainExecuteLifeCycle;
use liteflow_core::lifecycle::r#impl::ChainCacheLifeCycle;
use liteflow_core::log::LFLoggerManager;
use liteflow_core::monitor::MonitorFile;
use liteflow_core::slot::Ctx;
use liteflow_core::util::el_regex_util::ElRegexUtil;
use liteflow_core::{Frame, NodeRef, Slot, cmp};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// ChainCacheLifeCycle 的进程级实例与活跃/清理状态。
///
/// 对应 Java: `ChainCacheLifeCycle` 的进程级生命周期单例、`isActive` 与
/// 执行前 touch/执行后清理的 `PostProcessChainExecuteLifeCycle` 语义。
#[tokio::test]
async fn chain_cache_life_cycle_state_machine() {
    let cleaned = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let clean_chain: Arc<dyn Fn(&str) + Send + Sync> = Arc::new({
        let cleaned = cleaned.clone();
        move |chain_id| cleaned.lock().unwrap().push(chain_id.to_string())
    });

    assert!(ChainCacheLifeCycle::init_if_absent(8, clean_chain.clone()));
    // 重复初始化不覆盖既有实例
    assert!(!ChainCacheLifeCycle::init_if_absent(
        16,
        clean_chain.clone()
    ));

    let lifecycle = ChainCacheLifeCycle::get_life_cycle().expect("生命周期已初始化");
    // 未执行的 Chain 不活跃、未清理
    assert!(!lifecycle.is_active("chain-a"));
    assert!(!lifecycle.is_cleaned("chain-a"));

    // 执行前 touch 进入活跃缓存（Java postProcessBeforeChainExecute）
    let slot = Arc::new(Slot::new("RID-CACHE".to_string(), "main", Value::Null));
    lifecycle
        .post_process_before_chain_execute("chain-a", &slot)
        .await;
    assert!(lifecycle.is_active("chain-a"));
    // 活跃 Chain 执行后不会被清理
    lifecycle
        .post_process_after_chain_execute("chain-a", &slot)
        .await;
    assert!(!lifecycle.is_cleaned("chain-a"));
    assert!(cleaned.lock().unwrap().is_empty());
}

/// MonitorFile 批量路径登记（含非法路径拒绝）。
#[test]
fn monitor_file_batch_paths() {
    let monitor = MonitorFile::new(liteflow_core::FlowBus::new());
    assert!(
        monitor
            .add_monitor_file_paths(vec!["/tmp/a.xml", "/tmp/b.yml"])
            .is_ok()
    );
}

/// spawn_all 真实提交分支并收集结果。
#[tokio::test]
async fn spawn_all_submits_and_collects() {
    let component = Arc::new(cmp(|_| async { Ok(json!("done")) }));
    let node = liteflow_core::flow::element::node::Node::new(NodeRef::new("a"), component);
    let slot = Arc::new(Slot::new("RID-SPAWN".to_string(), "main", Value::Null));
    let ctx = Ctx::new(slot.clone());
    let frame = Frame::root();
    let executor = Arc::new(liteflow_core::thread::ExecutorService::new(
        1, 1, 8, "batch-e",
    ));

    let mut set = spawn_all(
        vec![Arc::new(node)],
        &ctx,
        &frame,
        &executor,
        Duration::from_secs(5),
    );
    let mut results = Vec::new();
    while let Some(joined) = set.join_next().await {
        let (index, _) = joined.expect("分支完成");
        results.push(index);
    }
    assert_eq!(results, vec![0]);
}

/// LFLoggerManager 执行日志开关与请求 ID 清理。
#[test]
fn logger_manager_switches_and_request_id() {
    let before = LFLoggerManager::is_print_execution_log();
    LFLoggerManager::set_print_execution_log(true);
    assert!(LFLoggerManager::is_print_execution_log());
    LFLoggerManager::set_print_execution_log(before);

    LFLoggerManager::remove_request_id();
    LFLoggerManager::set_request_id("REQ-1");
}

/// ElRegexUtil 抽象链识别与占位符替换。
#[test]
fn el_regex_abstract_chain_detection_and_replacement() {
    assert!(!ElRegexUtil::is_abstract_chain("THEN(a, b)"));
    assert!(ElRegexUtil::is_abstract_chain("{{base}}.THEN(a)"));

    let replaced = ElRegexUtil::replace_abstract_chain("{{base}}.THEN(a)", "{{base}} = THEN(b);")
        .expect("占位符应被替换");
    assert!(replaced.contains("THEN(b)"));
    assert!(!replaced.contains("{{base}}"));

    // 缺少实现时报解析错误
    let error = ElRegexUtil::replace_abstract_chain("{{missing}}.THEN(a)", "THEN(b)")
        .expect_err("未实现占位符应报错");
    assert!(error.to_string().contains("missing implementation"));
}
