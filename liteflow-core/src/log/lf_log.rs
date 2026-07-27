//! 对应 Java: com.yomahub.liteflow.log.LFLog

use log::Level;

use super::LFLoggerManager;

/// 为每条日志附加请求 ID 的日志包装器。
///
/// TRACE/DEBUG 直接交给底层 `log` 门面，INFO/WARN/ERROR 还受 LiteFlow 执行
/// 日志开关控制。对应 Java: `com.yomahub.liteflow.log.LFLog`。
#[derive(Debug, Clone)]
pub struct LFLog {
    target: String,
}

impl LFLog {
    /// 创建指定日志 target 的包装器。
    #[must_use]
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }

    /// 返回底层日志 target，对应 SLF4J `getName`。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.target
    }

    /// 返回底层日志名称。
    ///
    /// 返回值是 Rust `log` target，对应 SLF4J Logger 名称。对应 Java:
    /// `LFLog#getName`。
    #[must_use]
    pub fn get_name(&self) -> &str {
        self.name()
    }

    /// 返回 TRACE 级别是否启用。对应 Java: `LFLog#isTraceEnabled`。
    #[must_use]
    pub fn is_trace_enabled(&self) -> bool {
        log::log_enabled!(target: &self.target, Level::Trace)
    }

    /// 记录 TRACE 日志；与 Java 一致，不受执行日志开关影响。
    pub fn trace(&self, message: &str) {
        self.write(Level::Trace, message, false);
    }

    /// 记录 DEBUG 日志；与 Java 一致，不受执行日志开关影响。
    pub fn debug(&self, message: &str) {
        self.write(Level::Debug, message, false);
    }

    /// 返回 DEBUG 级别是否启用。对应 Java: `LFLog#isDebugEnabled`。
    #[must_use]
    pub fn is_debug_enabled(&self) -> bool {
        log::log_enabled!(target: &self.target, Level::Debug)
    }

    /// 记录 INFO 日志。
    pub fn info(&self, message: &str) {
        self.write(Level::Info, message, true);
    }

    /// 返回 INFO 级别是否启用。对应 Java: `LFLog#isInfoEnabled`。
    #[must_use]
    pub fn is_info_enabled(&self) -> bool {
        log::log_enabled!(target: &self.target, Level::Info)
    }

    /// 记录 WARN 日志。
    pub fn warn(&self, message: &str) {
        self.write(Level::Warn, message, true);
    }

    /// 返回 WARN 级别是否启用。对应 Java: `LFLog#isWarnEnabled`。
    #[must_use]
    pub fn is_warn_enabled(&self) -> bool {
        log::log_enabled!(target: &self.target, Level::Warn)
    }

    /// 记录 ERROR 日志。
    pub fn error(&self, message: &str) {
        self.write(Level::Error, message, true);
    }

    /// 返回 ERROR 级别是否启用。对应 Java: `LFLog#isErrorEnabled`。
    #[must_use]
    pub fn is_error_enabled(&self) -> bool {
        log::log_enabled!(target: &self.target, Level::Error)
    }

    fn write(&self, level: Level, message: &str, gated: bool) {
        if gated && !LFLoggerManager::is_print_execution_log() {
            return;
        }
        let prefix = LFLoggerManager::get_request_id()
            .filter(|request_id| !request_id.trim().is_empty())
            .map(|request_id| format!("[{request_id}]:"))
            .unwrap_or_default();
        log::log!(target: &self.target, level, "{prefix}{message}");
    }
}
