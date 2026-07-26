//! 多脚本语言与 Vernal 组合场景。

/// 逐一注册并核验全部受支持脚本语言。
pub async fn run_case() -> bool {
    liteflow_testcase_el_script_aviator_vernal::run_case().await
        && liteflow_testcase_el_script_graaljs_vernal::run_case().await
        && liteflow_testcase_el_script_groovy_vernal::run_case().await
        && liteflow_testcase_el_script_javascript_vernal::run_case().await
        && liteflow_testcase_el_script_lua_vernal::run_case().await
        && liteflow_testcase_el_script_python_vernal::run_case().await
        && liteflow_testcase_el_script_qlexpress_vernal::run_case().await
        && liteflow_testcase_el_script_rhai_vernal::run_case().await
}
