//! 28 个 testcase 子 crate 的聚合门禁。

#[tokio::test]
async fn all_twenty_eight_cases_have_executable_contracts() {
    let cases = [
        ("nospring", liteflow_testcase_el::nospring::run_case().await),
        ("vernal", liteflow_testcase_el::vernal::run_case().await),
        ("builder", liteflow_testcase_el::builder::run_case().await),
        (
            "routechain",
            liteflow_testcase_el::routechain::run_case().await,
        ),
        ("agent", liteflow_testcase_el::agent::run_case().await),
        (
            "declare_vernal",
            liteflow_testcase_el::declare_vernal::run_case().await,
        ),
        (
            "declare_multi_vernal",
            liteflow_testcase_el::declare_multi_vernal::run_case().await,
        ),
        ("apollo", liteflow_testcase_el::apollo::run_case().await),
        (
            "apollo_vernal",
            liteflow_testcase_el::apollo_vernal::run_case().await,
        ),
        ("etcd", liteflow_testcase_el::etcd::run_case().await),
        (
            "etcd_vernal",
            liteflow_testcase_el::etcd_vernal::run_case().await,
        ),
        ("nacos", liteflow_testcase_el::nacos::run_case().await),
        (
            "nacos_vernal",
            liteflow_testcase_el::nacos_vernal::run_case().await,
        ),
        ("redis", liteflow_testcase_el::redis::run_case().await),
        (
            "redis_vernal",
            liteflow_testcase_el::redis_vernal::run_case().await,
        ),
        ("sql", liteflow_testcase_el::sql::run_case().await),
        (
            "sql_vernal",
            liteflow_testcase_el::sql_vernal::run_case().await,
        ),
        ("zk", liteflow_testcase_el::zk::run_case().await),
        (
            "zk_vernal",
            liteflow_testcase_el::zk_vernal::run_case().await,
        ),
        (
            "script_aviator_vernal",
            liteflow_testcase_el::script_aviator_vernal::run_case().await,
        ),
        (
            "script_graaljs_vernal",
            liteflow_testcase_el::script_graaljs_vernal::run_case().await,
        ),
        (
            "script_groovy_vernal",
            liteflow_testcase_el::script_groovy_vernal::run_case().await,
        ),
        (
            "script_javascript_vernal",
            liteflow_testcase_el::script_javascript_vernal::run_case().await,
        ),
        (
            "script_lua_vernal",
            liteflow_testcase_el::script_lua_vernal::run_case().await,
        ),
        (
            "script_python_vernal",
            liteflow_testcase_el::script_python_vernal::run_case().await,
        ),
        (
            "script_qlexpress_vernal",
            liteflow_testcase_el::script_qlexpress_vernal::run_case().await,
        ),
        (
            "script_rhai_vernal",
            liteflow_testcase_el::script_rhai_vernal::run_case().await,
        ),
        (
            "script_multi_language_vernal",
            liteflow_testcase_el::script_multi_language_vernal::run_case().await,
        ),
    ];
    assert_eq!(cases.len(), 28);
    let failed = cases
        .into_iter()
        .filter_map(|(name, passed)| (!passed).then_some(name))
        .collect::<Vec<_>>();
    assert!(failed.is_empty(), "failed testcase crates: {failed:?}");
}
