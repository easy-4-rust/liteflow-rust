use std::sync::Arc;

use liteflow_core::flow::instance_id::NodeInstanceIdManageSpi;
use liteflow_core::{FlowBus, InstanceInfoDto, cmp};
use liteflow_rule_sql::{
    ChainRead, ChainReadPollTask, JDBCHelper, LiteflowDataSourceConnectFactory,
    SQLParserClassNameSpi, SQLParserVO, SQLXmlELParser, ScriptRead, ScriptReadPollTask,
    SqlNodeInstanceIdManageSpiImpl, SqlRead, SqlReadPollTask,
};
use rusqlite::Connection;
use serde_json::{Value, json};

fn sqlite_config(path: &str) -> SQLParserVO {
    SQLParserVO::sqlite(path.to_string())
}

#[test]
fn parser_vo_uses_java_camel_case_and_preserves_defaults() {
    let config: SQLParserVO = serde_json::from_str(
        r#"{
            "url":"jdbc:sqlite:/tmp/liteflow.db",
            "driverClassName":"org.sqlite.JDBC",
            "username":"",
            "password":"",
            "applicationName":"demo",
            "chainTableName":"lf_chain",
            "pollingEnabled":true
        }"#,
    )
    .unwrap();

    assert_eq!(config.chain_name_field, "chain_name");
    assert_eq!(config.el_data_field, "el_data");
    assert_eq!(config.polling_interval_seconds, 60);
    assert!(config.polling_enabled);
    assert!(config.is_use_jdbc_conn());
    assert_eq!(
        SQLParserClassNameSpi.get_spi_class_name(),
        "liteflow_rule_sql::SQLXmlELParser"
    );
}

#[tokio::test]
async fn custom_field_mapping_filters_application_and_disabled_rows() {
    let database = tempfile::NamedTempFile::new().unwrap();
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE lf_chain (
                app TEXT NOT NULL,
                chain_key TEXT NOT NULL,
                namespace_key TEXT,
                route_el TEXT,
                body_el TEXT NOT NULL,
                enabled INTEGER NOT NULL
            );
            CREATE TABLE lf_script (
                app TEXT NOT NULL,
                script_key TEXT NOT NULL,
                display_name TEXT,
                node_type TEXT NOT NULL,
                lang TEXT,
                source TEXT NOT NULL,
                enabled INTEGER NOT NULL
            );
            INSERT INTO lf_chain VALUES
                ('demo','mappedChain','ns-a',NULL,'THEN(mappedNode)',1),
                ('demo','disabledChain','ns-a',NULL,'THEN(mappedNode)',0),
                ('other','otherChain','ns-b',NULL,'THEN(mappedNode)',1);
            INSERT INTO lf_script VALUES
                ('demo','mappedScript','Mapped Script','script','rhai','40 + 2',1),
                ('demo','disabledScript','Disabled','script','rhai','1 + 1',0);
            "#,
        )
        .unwrap();
    drop(connection);

    let mut config = SQLParserVO {
        url: Some(database.path().to_string_lossy().into_owned()),
        driver_class_name: Some("org.sqlite.JDBC".to_string()),
        username: Some(String::new()),
        password: Some(String::new()),
        application_name: Some("demo".to_string()),
        chain_table_name: Some("lf_chain".to_string()),
        script_table_name: Some("lf_script".to_string()),
        ..SQLParserVO::default()
    };
    config.chain_application_name_field = "app".to_string();
    config.chain_name_field = "chain_key".to_string();
    config.namespace_field = Some("namespace_key".to_string());
    config.route_field = Some("route_el".to_string());
    config.el_data_field = "body_el".to_string();
    config.chain_enable_field = Some("enabled".to_string());
    config.script_application_name_field = "app".to_string();
    config.script_id_field = "script_key".to_string();
    config.script_name_field = "display_name".to_string();
    config.script_type_field = "node_type".to_string();
    config.script_language_field = Some("lang".to_string());
    config.script_data_field = "source".to_string();
    config.script_enable_field = Some("enabled".to_string());

    let parser = SQLXmlELParser::new(config).unwrap();
    let xml = parser.parse_custom().unwrap();
    assert!(xml.contains("mappedChain"));
    assert!(xml.contains("mappedScript"));
    assert!(!xml.contains("disabledChain"));
    assert!(!xml.contains("disabledScript"));
    assert!(!xml.contains("otherChain"));

    let bus = FlowBus::new();
    bus.register(
        "mappedNode",
        cmp(|ctx| async move {
            ctx.set_data("mapped", json!(true));
            Ok(Value::Null)
        }),
    );
    liteflow_core::parser::el::load_xml_str(&bus, &xml).unwrap();
    let response = bus.execute("mappedChain").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("mapped"), Some(json!(true)));
}

#[test]
fn named_and_auto_lookup_data_sources_select_real_databases() {
    let baomidou_db = tempfile::NamedTempFile::new().unwrap();
    let sharding_db = tempfile::NamedTempFile::new().unwrap();
    let unrelated_db = tempfile::NamedTempFile::new().unwrap();
    let auto_db = tempfile::NamedTempFile::new().unwrap();
    Connection::open(auto_db.path())
        .unwrap()
        .execute_batch("CREATE TABLE auto_chain(chain_name TEXT, el_data TEXT);")
        .unwrap();

    LiteflowDataSourceConnectFactory::register_data_source(
        "baomidou-main",
        baomidou_db.path().to_string_lossy(),
    );
    LiteflowDataSourceConnectFactory::register_data_source(
        "sharding-main",
        sharding_db.path().to_string_lossy(),
    );
    LiteflowDataSourceConnectFactory::register_data_source(
        "00-unrelated",
        unrelated_db.path().to_string_lossy(),
    );
    LiteflowDataSourceConnectFactory::register_data_source(
        "99-auto-liteflow",
        auto_db.path().to_string_lossy(),
    );

    let baomidou_config = SQLParserVO {
        baomidou_data_source: Some("baomidou-main".to_string()),
        ..SQLParserVO::default()
    };
    let connector = LiteflowDataSourceConnectFactory::get_connect(&baomidou_config).unwrap();
    assert_eq!(connector.name(), "BaoMiDouDynamicDsConn");
    connector.get_conn(&baomidou_config).unwrap();

    let sharding_config = SQLParserVO {
        sharding_jdbc_data_source: Some("sharding-main".to_string()),
        ..SQLParserVO::default()
    };
    let connector = LiteflowDataSourceConnectFactory::get_connect(&sharding_config).unwrap();
    assert_eq!(connector.name(), "ShardingJdbcDsConn");
    connector.get_conn(&sharding_config).unwrap();

    let auto_config = SQLParserVO {
        chain_table_name: Some("auto_chain".to_string()),
        ..SQLParserVO::default()
    };
    let connector = LiteflowDataSourceConnectFactory::get_connect(&auto_config).unwrap();
    assert_eq!(connector.name(), "LiteFlowAutoLookUpJdbcConn");
    let connection = connector.get_conn(&auto_config).unwrap();
    connection
        .prepare("SELECT chain_name,el_data FROM auto_chain")
        .unwrap();
}

#[tokio::test]
async fn polling_reconciles_chain_and_script_add_update_delete() {
    let database = tempfile::NamedTempFile::new().unwrap();
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE chain (
                chain_id TEXT NOT NULL,
                namespace TEXT,
                el_data TEXT NOT NULL,
                route TEXT,
                enable INTEGER
            );
            CREATE TABLE script (
                node_id TEXT NOT NULL,
                name TEXT,
                script_type TEXT NOT NULL,
                language TEXT,
                script TEXT NOT NULL
            );
            INSERT INTO chain VALUES ('pollChain','DEFAULT','THEN(a)',NULL,1);
            INSERT INTO script VALUES ('sqlScript','SQL Script','script','rhai','40 + 2');
            "#,
        )
        .unwrap();
    drop(connection);

    let config = sqlite_config(&database.path().to_string_lossy());
    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::String("a".to_string())) }));
    bus.register(
        "b",
        cmp(|ctx| async move {
            ctx.set_data("used_b", json!(true));
            Ok(Value::Null)
        }),
    );

    let chain_read = Arc::new(ChainRead::new(config.clone()));
    let chain_task = ChainReadPollTask::new(chain_read.clone(), bus.clone());
    let initial_chains = chain_read.read().unwrap();
    chain_task.do_save(&initial_chains).unwrap();
    chain_task.init_data(&initial_chains);

    let script_read = Arc::new(ScriptRead::new(config.clone()));
    let script_task = ScriptReadPollTask::new(script_read.clone(), bus.clone());
    let initial_scripts = script_read.read().unwrap();
    script_task.do_save(&initial_scripts).unwrap();
    script_task.init_data(&initial_scripts);
    assert!(bus.contains_node("sqlScript"));

    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute(
            "UPDATE chain SET el_data='THEN(b)' WHERE chain_id='pollChain'",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO chain VALUES ('addedChain','DEFAULT','THEN(b)',NULL,1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE script SET script='41 + 1' WHERE node_id='sqlScript'",
            [],
        )
        .unwrap();
    drop(connection);

    chain_task.execute().unwrap();
    script_task.execute().unwrap();
    assert!(bus.contains_chain("addedChain"));
    let response = bus.execute("pollChain").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("used_b"), Some(json!(true)));

    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute("DELETE FROM chain WHERE chain_id='pollChain'", [])
        .unwrap();
    connection
        .execute("DELETE FROM script WHERE node_id='sqlScript'", [])
        .unwrap();
    drop(connection);

    chain_task.execute().unwrap();
    script_task.execute().unwrap();
    assert!(!bus.contains_chain("pollChain"));
    assert!(!bus.contains_node("sqlScript"));
}

#[tokio::test]
async fn parser_managed_scheduler_polls_and_can_be_cancelled() {
    let database = tempfile::NamedTempFile::new().unwrap();
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE chain (
                chain_id TEXT NOT NULL,
                namespace TEXT,
                el_data TEXT NOT NULL,
                route TEXT,
                enable INTEGER
            );
            CREATE TABLE script (
                node_id TEXT NOT NULL,
                name TEXT,
                script_type TEXT NOT NULL,
                language TEXT,
                script TEXT NOT NULL
            );
            INSERT INTO chain VALUES ('scheduledChain','DEFAULT','THEN(a)',NULL,1);
            "#,
        )
        .unwrap();
    drop(connection);

    let mut config = sqlite_config(&database.path().to_string_lossy());
    config.polling_enabled = true;
    config.polling_start_seconds = 1;
    config.polling_interval_seconds = 1;
    let parser = SQLXmlELParser::new(config).unwrap();

    let bus = FlowBus::new();
    bus.register("a", cmp(|_| async { Ok(Value::Null) }));
    bus.register(
        "b",
        cmp(|ctx| async move {
            ctx.set_data("scheduled_b", json!(true));
            Ok(Value::Null)
        }),
    );
    liteflow_core::parser::el::load_xml_str(&bus, &parser.parse_custom().unwrap()).unwrap();
    let handle = parser.start_polling(bus.clone()).unwrap();

    Connection::open(database.path())
        .unwrap()
        .execute(
            "UPDATE chain SET el_data='THEN(b)' WHERE chain_id='scheduledChain'",
            [],
        )
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1_250)).await;

    let response = bus.execute("scheduledChain").await;
    assert!(response.is_success(), "{}", response.message);
    assert_eq!(response.data("scheduled_b"), Some(json!(true)));
    handle.abort();
    assert!(handle.await.unwrap_err().is_cancelled());
}

#[test]
fn instance_id_spi_creates_reads_and_updates_real_table() {
    let database = tempfile::NamedTempFile::new().unwrap();
    let mut config = sqlite_config(&database.path().to_string_lossy());
    config.application_name = Some("instance-app".to_string());
    let spi = SqlNodeInstanceIdManageSpiImpl::new(config.clone());
    spi.create_table().unwrap();

    let first = vec![InstanceInfoDto::new("chain-a", "node-a", "node-a-0", 0)];
    spi.write_instance_id_file(&first, "md5-first", "chain-a")
        .unwrap();
    let lines = spi.read_instance_id_file("chain-a").unwrap();
    assert_eq!(lines[0], "md5-first");
    assert!(lines[1].contains("node-a-0"));

    let second = vec![
        InstanceInfoDto::new("chain-a", "node-a", "node-a-0", 0),
        InstanceInfoDto::new("chain-a", "node-a", "node-a-1", 1),
    ];
    spi.write_instance_id_file(&second, "md5-second", "chain-a")
        .unwrap();
    let lines = spi.read_instance_id_file("chain-a").unwrap();
    assert_eq!(lines[0], "md5-second");
    assert!(lines[1].contains("node-a-1"));

    let generated = spi.gen_instance_id("chain-a", "node-a", 2);
    assert_eq!(generated, spi.gen_instance_id("chain-a", "node-a", 2));

    let helper = JDBCHelper::init(config);
    helper.create_node_instance_id_table().unwrap();
}
