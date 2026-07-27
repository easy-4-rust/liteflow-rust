use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::ShellMode;

/// Agent 内置 Shell 工具的安全配置。
///
/// 命令执行前按首个 token 做白名单/黑名单检查，执行时应用超时和最大输出限制，
/// 防止模型触发危险命令或把超大输出写入上下文。
///
/// 对应 Java: `com.yomahub.liteflow.property.agent.ShellConfig`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShellConfig {
    /// 命令过滤模式。
    pub mode: ShellMode,
    /// 白名单模式允许的命令首 token。
    pub whitelist: Vec<String>,
    /// 黑名单模式禁止的命令首 token。
    pub blacklist: Vec<String>,
    /// 单条命令最大执行时长。
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// 单次命令最大输出字节数。
    pub max_output_bytes: u64,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            mode: ShellMode::Whitelist,
            whitelist: [
                "ls",
                "find",
                "tree",
                "stat",
                "file",
                "basename",
                "dirname",
                "pwd",
                "which",
                "cat",
                "head",
                "tail",
                "grep",
                "sed",
                "awk",
                "wc",
                "sort",
                "uniq",
                "cut",
                "tr",
                "diff",
                "echo",
                "printf",
                "expr",
                "date",
                "whoami",
                "hostname",
                "uname",
                "env",
                "df",
                "du",
                "ps",
                "md5sum",
                "sha256sum",
                "jq",
                "curl",
                "wget",
                "python3",
                "node",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            blacklist: ["rm", "sudo", "shutdown", "mkfs", "dd"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            timeout: Duration::from_secs(30),
            max_output_bytes: 1024 * 1024,
        }
    }
}

impl ShellConfig {
    /// 返回命令过滤模式。对应 Java: `ShellConfig#getMode`。
    #[must_use]
    pub fn mode(&self) -> ShellMode {
        self.mode
    }

    /// 返回命令过滤模式。
    ///
    /// 返回值决定首 token 按白名单、黑名单或关闭策略检查。对应 Java:
    /// `ShellConfig#getMode`。
    #[must_use]
    pub fn get_mode(&self) -> ShellMode {
        self.mode()
    }

    /// 设置命令过滤模式。对应 Java: `ShellConfig#setMode`。
    pub fn set_mode(&mut self, mode: ShellMode) {
        self.mode = mode;
    }

    /// 返回白名单。对应 Java: `ShellConfig#getWhitelist`。
    #[must_use]
    pub fn whitelist(&self) -> &[String] {
        &self.whitelist
    }

    /// 返回白名单模式允许的命令首 token 列表。
    ///
    /// 对应 Java: `ShellConfig#getWhitelist`。
    #[must_use]
    pub fn get_whitelist(&self) -> &[String] {
        self.whitelist()
    }

    /// 设置白名单。对应 Java: `ShellConfig#setWhitelist`。
    pub fn set_whitelist(&mut self, whitelist: Vec<String>) {
        self.whitelist = whitelist;
    }

    /// 返回黑名单。对应 Java: `ShellConfig#getBlacklist`。
    #[must_use]
    pub fn blacklist(&self) -> &[String] {
        &self.blacklist
    }

    /// 返回黑名单模式拒绝的命令首 token 列表。
    ///
    /// 对应 Java: `ShellConfig#getBlacklist`。
    #[must_use]
    pub fn get_blacklist(&self) -> &[String] {
        self.blacklist()
    }

    /// 设置黑名单。对应 Java: `ShellConfig#setBlacklist`。
    pub fn set_blacklist(&mut self, blacklist: Vec<String>) {
        self.blacklist = blacklist;
    }

    /// 返回命令超时。对应 Java: `ShellConfig#getTimeout`。
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// 返回单条命令最大执行时长。
    ///
    /// 超时后 ManagedShellCommandTool 会终止子进程。对应 Java:
    /// `ShellConfig#getTimeout`。
    #[must_use]
    pub fn get_timeout(&self) -> Duration {
        self.timeout()
    }

    /// 设置命令超时。对应 Java: `ShellConfig#setTimeout`。
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// 返回最大输出字节数。对应 Java: `ShellConfig#getMaxOutputBytes`。
    #[must_use]
    pub fn max_output_bytes(&self) -> u64 {
        self.max_output_bytes
    }

    /// 返回单次命令最大输出字节数。
    ///
    /// 达到上限后 stdout 会被截断。对应 Java:
    /// `ShellConfig#getMaxOutputBytes`。
    #[must_use]
    pub fn get_max_output_bytes(&self) -> u64 {
        self.max_output_bytes()
    }

    /// 设置最大输出字节数。对应 Java: `ShellConfig#setMaxOutputBytes`。
    pub fn set_max_output_bytes(&mut self, max_output_bytes: u64) {
        self.max_output_bytes = max_output_bytes;
    }
}
