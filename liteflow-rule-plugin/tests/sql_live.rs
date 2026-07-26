#![cfg(feature = "sql")]

use std::sync::Arc;

use liteflow_core::rule_plugin::{RuleSource, RuleSourceWatcher};
use liteflow_core::{FlowBus, cmp};
use liteflow_rule_plugin::sql::SqlRuleSource;
use serde_json::{Value, json};

#[tokio::test]
async fn sqlite_source_fetches_loads_and_executes_real_rule() {
    let database = tempfile::NamedTempFile::new().unwrap();
    let connection = rusqlite::Connection::open(database.path()).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE chain (
                chain_id TEXT NOT NULL,
                namespace TEXT,
                el_data TEXT,
                route TEXT,
                body TEXT,
                enable INTEGER
            );
            CREATE TABLE script (
                node_id TEXT NOT NULL,
                script_type TEXT,
                language TEXT,
                script TEXT NOT NULL
            );
            INSERT INTO chain(chain_id, namespace, el_data, enable)
            VALUES ('sqlChain', 'DEFAULT', 'THEN(sqlNode)', 1);
            "#,
        )
        .unwrap();
    drop(connection);

    let source = SqlRuleSource::new(database.path().to_string_lossy());
    let (text, fingerprint) = source.fetch().await.unwrap();
    assert!(text.contains("sqlChain"));
    assert!(!fingerprint.is_empty());

    let bus = FlowBus::new();
    bus.register(
        "sqlNode",
        cmp(|ctx| async move {
            ctx.set_data("sql_loaded", json!(true));
            Ok(Value::Null)
        }),
    );
    let _watcher = RuleSourceWatcher::new(bus.clone(), Arc::new(source))
        .await
        .unwrap();

    let response = bus.execute("sqlChain").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("sql_loaded"), Some(json!(true)));
}
