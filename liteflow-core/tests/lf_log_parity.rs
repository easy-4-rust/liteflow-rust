//! Java `LFLog` 名称和级别开关语义回归测试。

use liteflow_core::log::LFLog;
use log::Level;

#[test]
fn java_named_level_queries_delegate_to_the_real_log_facade() {
    let logger = LFLog::new("liteflow::parity");

    assert_eq!(logger.get_name(), "liteflow::parity");
    assert_eq!(
        logger.is_trace_enabled(),
        log::log_enabled!(target: "liteflow::parity", Level::Trace)
    );
    assert_eq!(
        logger.is_debug_enabled(),
        log::log_enabled!(target: "liteflow::parity", Level::Debug)
    );
    assert_eq!(
        logger.is_info_enabled(),
        log::log_enabled!(target: "liteflow::parity", Level::Info)
    );
    assert_eq!(
        logger.is_warn_enabled(),
        log::log_enabled!(target: "liteflow::parity", Level::Warn)
    );
    assert_eq!(
        logger.is_error_enabled(),
        log::log_enabled!(target: "liteflow::parity", Level::Error)
    );
}
