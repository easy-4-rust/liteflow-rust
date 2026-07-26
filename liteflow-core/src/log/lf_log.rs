//! 对应 Java: com.yomahub.liteflow.log.LFLog

use log::Level;

use super::LFLoggerManager;

/// 为每条日志附加请求 id，并服从执行日志开关的日志包装器。
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

    /// 记录 TRACE 日志；与 Java 一致，不受执行日志开关影响。
    pub fn trace(&self, message: &str) {
        self.write(Level::Trace, message, false);
    }

    /// 记录 DEBUG 日志；与 Java 一致，不受执行日志开关影响。
    pub fn debug(&self, message: &str) {
        self.write(Level::Debug, message, false);
    }

    /// 记录 INFO 日志。
    pub fn info(&self, message: &str) {
        self.write(Level::Info, message, true);
    }

    /// 记录 WARN 日志。
    pub fn warn(&self, message: &str) {
        self.write(Level::Warn, message, true);
    }

    /// 记录 ERROR 日志。
    pub fn error(&self, message: &str) {
        self.write(Level::Error, message, true);
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
