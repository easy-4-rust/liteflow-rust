//! Chain 常量。
//!
//! 对应 Java: `com.yomahub.liteflow.common.ChainConstant`。

/// 汇总规则结构、EL 操作符和运行期上下文使用的稳定常量。
///
/// Java 使用 interface 字段承载常量；Rust 使用无实例单元类型的关联常量，
/// 避免把常量散落到解析器和执行器中。
pub struct ChainConstant;

impl ChainConstant {
    pub const PARALLEL: &'static str = "parallel";
    pub const CHAIN: &'static str = "chain";
    pub const ROUTE: &'static str = "route";
    pub const BODY: &'static str = "body";
    pub const FLOW: &'static str = "flow";
    pub const NODES: &'static str = "nodes";
    pub const NODE: &'static str = "node";
    pub const ID: &'static str = "id";
    pub const CLASS: &'static str = "class";
    pub const FILE: &'static str = "file";
    pub const NAME: &'static str = "name";
    pub const ENABLE: &'static str = "enable";
    pub const LANGUAGE: &'static str = "language";
    pub const NAMESPACE: &'static str = "namespace";
    pub const THREAD_POOL_EXECUTOR_CLASS: &'static str = "thread-pool-executor-class";
    pub const DEFAULT_NAMESPACE: &'static str = "default";
    pub const VALUE: &'static str = "value";
    pub const ANY: &'static str = "any";
    pub const MUST: &'static str = "must";
    pub const PERCENTAGE: &'static str = "percentage";
    pub const TYPE: &'static str = "type";
    pub const THEN: &'static str = "THEN";
    pub const WHEN: &'static str = "WHEN";
    pub const SER: &'static str = "SER";
    pub const PAR: &'static str = "PAR";
    pub const SWITCH: &'static str = "SWITCH";
    pub const PRE: &'static str = "PRE";
    pub const FINALLY: &'static str = "FINALLY";
    pub const IF: &'static str = "IF";
    pub const ELSE: &'static str = "ELSE";
    pub const ELIF: &'static str = "ELIF";
    pub const TO: &'static str = "TO";
    pub const TAG: &'static str = "tag";
    pub const IGNORE_ERROR: &'static str = "ignoreError";
    pub const THREAD_POOL: &'static str = "threadPool";
    pub const WHILE: &'static str = "WHILE";
    pub const FOR: &'static str = "FOR";
    pub const DO: &'static str = "DO";
    pub const BREAK: &'static str = "BREAK";
    pub const DATA: &'static str = "data";
    pub const ITERATOR: &'static str = "ITERATOR";
    pub const MONITOR_BUS: &'static str = "monitorBus";
    pub const CURR_CHAIN_ID: &'static str = "currChainId";
    pub const DEFAULT: &'static str = "DEFAULT";
    pub const CATCH: &'static str = "CATCH";
    pub const AND: &'static str = "AND";
    pub const OR: &'static str = "OR";
    pub const NOT: &'static str = "NOT";
    pub const MAX_WAIT_SECONDS: &'static str = "maxWaitSeconds";
    pub const MAX_WAIT_MILLISECONDS: &'static str = "maxWaitMilliseconds";
    pub const EXTENDS: &'static str = "extends";
    pub const RETRY: &'static str = "retry";
    pub const NODE_INSTANCE_PATH: &'static str = ".node_instance_id";
    pub const USER_DIR: &'static str = "user.dir";
    pub const BIND: &'static str = "bind";
    pub const CONTEXT_SEARCH_REGEX: &'static str = r"^\$\{(.*?)\}$";
}
