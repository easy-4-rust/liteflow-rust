//! SQL 节点实例编号 SPI 与标准 Chain Builder 的真实集成验收。

use std::sync::Arc;

use liteflow_core::flow::instance_id::NodeInstanceIdManageSpi;
use liteflow_core::{FlowBus, LiteflowConfig, LiteflowConfigGetter, cmp};
use liteflow_rule_sql::{
    ChainRead, ChainReadPollTask, SQLParserVO, SqlNodeInstanceIdManageSpiImpl, SqlRead,
    SqlReadPollTask,
};
use rusqlite::Connection;
use serde_json::Value;

fn register_null_component(bus: &FlowBus) {
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
}

fn response_instance_ids(response: &liteflow_core::LiteflowResponse) -> Vec<Option<String>> {
    response
        .steps
        .iter()
        .map(|step| step.get_node_instance_id().map(ToOwned::to_owned))
        .collect()
}

/// 验证 SQL SPI 对同 MD5 恢复编号，并在 EL 变化时生成新编号后真实 upsert。
///
/// 对应 Java:
/// `SqlNodeInstanceIdManageSpiImpl#readInstanceIdFile`、
/// `SqlNodeInstanceIdManageSpiImpl#writeInstanceIdFile` 与
/// `BaseNodeInstanceIdManageSpi#setNodesInstanceId`。
#[tokio::test]
async fn sql_spi_regenerates_changed_el_and_restores_matching_snapshot() {
    LiteflowConfigGetter::clean();
    let mut liteflow_config = LiteflowConfig::default();
    liteflow_config.set_enable_node_instance_id(true);
    LiteflowConfigGetter::set_liteflow_config(liteflow_config);

    let database = tempfile::NamedTempFile::new().expect("应创建隔离的 SQLite 文件");
    Connection::open(database.path())
        .expect("应打开隔离的 SQLite 文件")
        .execute_batch(
            r#"
            CREATE TABLE chain (
                application_name TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                namespace TEXT,
                el_data TEXT NOT NULL,
                route TEXT,
                enable INTEGER
            );
            INSERT INTO chain VALUES (
                'instance-builder-app',
                'sql-instance-chain',
                'instance',
                'THEN(a, a)',
                'route_check',
                1
            );
            "#,
        )
        .expect("应创建并写入真实 SQL Chain 表");
    let mut sql_config = SQLParserVO::sqlite(database.path().to_string_lossy().into_owned());
    sql_config.application_name = Some("instance-builder-app".to_string());

    let first_spi = Arc::new(SqlNodeInstanceIdManageSpiImpl::new(sql_config.clone()));
    first_spi
        .create_table()
        .expect("应创建真实 instanceId 数据表");
    let first_bus = FlowBus::new();
    first_bus.set_instance_id_spi(first_spi.clone());
    register_null_component(&first_bus);
    first_bus.register("route_check", cmp(|_| async { Ok(Value::Bool(true)) }));
    let first_read = Arc::new(ChainRead::new(sql_config.clone()));
    let first_task = ChainReadPollTask::new(first_read.clone(), first_bus.clone());
    let first_rows = first_read.read().expect("应读取首次 SQL Chain 快照");
    first_task
        .do_save(&first_rows)
        .expect("SQL 轮询首次构建应把编号写入 SQL");
    first_task.init_data(&first_rows);
    let mut first_responses = first_bus
        .execute_route_chain(Some("instance"), Value::Null)
        .await
        .expect("首次 SQL route Chain 应命中");
    let first_response = first_responses.remove(0);
    assert!(first_response.is_success(), "{}", first_response.message);
    let first_ids = response_instance_ids(&first_response);
    assert_eq!(first_ids.len(), 2);
    assert!(first_ids.iter().all(Option::is_some));

    // EL 摘要变化时 Java 基类会重新生成全部短 UUID；SQL SPI 只负责 upsert，
    // 不能通过进程内 `(chain,node,index)` 缓存继续复用旧编号。
    Connection::open(database.path())
        .expect("应重新打开 SQLite 文件")
        .execute(
            "UPDATE chain SET el_data='THEN(a, a, a)' WHERE chain_id='sql-instance-chain'",
            [],
        )
        .expect("应更新 SQL Chain 主体");
    first_task
        .execute()
        .expect("轮询应识别 EL 变化并通过标准 Builder 更新 Chain");
    let mut changed_responses = first_bus
        .execute_route_chain(Some("instance"), Value::Null)
        .await
        .expect("更新后的 SQL route Chain 应命中");
    let changed_response = changed_responses.remove(0);
    assert!(
        changed_response.is_success(),
        "{}",
        changed_response.message
    );
    let changed_ids = response_instance_ids(&changed_response);
    assert_eq!(changed_ids.len(), 3);
    assert_ne!(changed_ids[0], first_ids[0]);
    assert_ne!(changed_ids[1], first_ids[1]);

    let persisted_lines = first_spi
        .read_instance_id_file("sql-instance-chain")
        .expect("应从真实 SQL 表读取更新后的两行快照");
    assert_eq!(persisted_lines.len(), 2);
    for instance_id in changed_ids.iter().flatten() {
        assert!(persisted_lines[1].contains(instance_id));
    }
    assert!(
        !persisted_lines[1].contains("route_check"),
        "Java 只给 route Chain 的主体 Condition 分配实例编号"
    );

    // 使用全新的 FlowBus 与 SQL SPI，证明相同 MD5 的稳定性来自数据库快照，
    // 而不是旧实例或进程缓存。
    let restored_config = sql_config.clone();
    let restored_spi = Arc::new(SqlNodeInstanceIdManageSpiImpl::new(sql_config));
    let restored_bus = FlowBus::new();
    restored_bus.set_instance_id_spi(restored_spi);
    register_null_component(&restored_bus);
    restored_bus.register("route_check", cmp(|_| async { Ok(Value::Bool(true)) }));
    let restored_read = Arc::new(ChainRead::new(restored_config));
    let restored_task = ChainReadPollTask::new(restored_read.clone(), restored_bus.clone());
    restored_task
        .do_save(
            &restored_read
                .read()
                .expect("全新轮询任务应读取当前 SQL Chain"),
        )
        .expect("相同 EL 应从 SQL 恢复编号");
    let mut restored_responses = restored_bus
        .execute_route_chain(Some("instance"), Value::Null)
        .await
        .expect("恢复后的 SQL route Chain 应命中");
    let restored_response = restored_responses.remove(0);
    assert!(
        restored_response.is_success(),
        "{}",
        restored_response.message
    );
    assert_eq!(response_instance_ids(&restored_response), changed_ids);

    LiteflowConfigGetter::clean();
}
