use crate::common::entity::ValidationResp;
use crate::enums::ScriptTypeEnum;
use crate::exception::LFResult;
use crate::slot::CmpContext;
use serde_json::Value;

/// 脚本执行器抽象，统一脚本的加载、执行、卸载与缓存生命周期。
///
/// Java 抽象类把编译产物声明为 `Object`；Rust 端由各具体执行器在自身文件中保存
/// 强类型编译产物，并通过本 trait 暴露对象安全的生命周期契约。
///
/// 对应 Java: `com.yomahub.liteflow.script.ScriptExecutor`。
pub trait ScriptExecutor: Send + Sync {
    /// 初始化脚本执行器及其底层引擎。
    ///
    /// 返回初始化结果。对应 Java: `ScriptExecutor#init`。
    fn init(&self) -> LFResult<()> {
        Ok(())
    }

    /// 编译并加载指定节点的脚本。
    ///
    /// `node_id` 是脚本节点标识，`script` 是脚本文本。对应 Java:
    /// `ScriptExecutor#load`。
    fn load(&self, node_id: &str, script: &str) -> LFResult<()>;

    /// 执行需要合并编译的第二阶段加载。
    ///
    /// 普通执行器无需第二阶段，默认直接成功。对应 Java:
    /// `ScriptExecutor#loadSecondPhase`。
    fn load_second_phase(&self) -> LFResult<()> {
        Ok(())
    }

    /// 从执行器缓存中卸载指定节点脚本。
    ///
    /// `node_id` 只标识脚本，不删除 LiteFlow 节点。对应 Java:
    /// `ScriptExecutor#unLoad`。
    fn unload(&self, node_id: &str) -> LFResult<()>;

    /// 返回当前执行器已经加载的全部节点 id。
    ///
    /// 对应 Java: `ScriptExecutor#getNodeIds`。
    fn node_ids(&self) -> LFResult<Vec<String>>;

    /// 执行指定节点已经加载的脚本。
    ///
    /// `node_id` 选择缓存脚本，`ctx` 提供流程上下文。该入口保留 Java
    /// `execute` 的统一委托语义。对应 Java: `ScriptExecutor#execute`。
    fn execute(&self, node_id: &str, ctx: &CmpContext) -> LFResult<Value> {
        self.execute_script(node_id, ctx)
    }

    /// 执行具体脚本引擎中的已编译脚本。
    ///
    /// 返回可跨引擎传递的 JSON 值。对应 Java:
    /// `ScriptExecutor#executeScript`。
    fn execute_script(&self, node_id: &str, ctx: &CmpContext) -> LFResult<Value>;

    /// 清理当前执行器的全部脚本缓存。
    ///
    /// 对应 Java: `ScriptExecutor#cleanCache`。
    fn clean_cache(&self) -> LFResult<()>;

    /// 返回执行器支持的脚本语言类型。
    ///
    /// 对应 Java: `ScriptExecutor#scriptType`。
    fn script_type(&self) -> ScriptTypeEnum;

    /// 校验脚本能否被当前引擎编译。
    ///
    /// `script` 是待校验文本；只返回成功与否。对应 Java:
    /// `ScriptExecutor#validate`。
    fn validate(&self, script: &str) -> bool {
        self.validate_with_ex(script).is_success()
    }

    /// 校验脚本并保留编译失败原因。
    ///
    /// 对应 Java: `ScriptExecutor#validate` 的 `ValidationResp` 返回值。
    fn validate_with_ex(&self, script: &str) -> ValidationResp;
}
