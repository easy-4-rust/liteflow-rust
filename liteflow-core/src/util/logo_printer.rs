//! 对应 Java: com.yomahub.liteflow.util.LOGOPrinter

use crate::log::LFLoggerManager;

/// LiteFlow 启动 Logo 打印器。
pub struct LOGOPrinter;

impl LOGOPrinter {
    /// 通过 LiteFlow 日志门面打印版本与项目地址。
    pub fn print() {
        LFLoggerManager::get_logger("liteflow::LOGOPrinter").info(&Self::logo());
    }

    /// 生成完整 Logo 文本，便于启动器复用与测试。
    #[must_use]
    pub fn logo() -> String {
        format!(
            "\n================================================================================================\n\
             \t\t _     ___ _____ _____      _____ _     _____        __\n\
             \t\t| |   |_ _|_   _| ____|    |  ___| |   / _ \\\\ \\\\      / /\n\
             \t\t| |    | |  | | |  _| _____| |_  | |  | | | \\\\ \\\\ /\\\\ / /\n\
             \t\t| |___ | |  | | | |__|_____|  _| | |__| |_| |\\\\ V  V /\n\
             \t\t|_____|___| |_| |_____|    |_|   |_____\\\\___/  \\\\_/\\\\_/\n\n\
             \t\tVersion: {}\n\
             \t\tMake your code amazing.\n\
             \t\twebsite：https://liteflow.cc\n\
             ================================================================================================\n",
            env!("CARGO_PKG_VERSION")
        )
    }
}
