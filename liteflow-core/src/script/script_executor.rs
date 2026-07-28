use crate::common::entity::ValidationResp;
use crate::enums::ScriptTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::lifecycle::LifeCycleHolder;
use crate::script::ScriptExecuteWrap;
use crate::slot::CmpContext;
use crate::spi::CmpAroundAspectHolder;
use serde_json::{Map, Number, Value};
use std::sync::Arc;

/// 脚本执行器抽象，统一脚本的加载、执行、卸载与缓存生命周期。
///
/// Java 抽象类把编译产物声明为 `Object`；Rust 端由各具体执行器在自身文件中保存
/// 强类型编译产物，并通过本 trait 暴露对象安全的生命周期契约。
///
/// 对应 Java: `com.yomahub.liteflow.script.ScriptExecutor`。
pub trait ScriptExecutor: Send + Sync {
    /// 初始化脚本执行器及其底层引擎。
    ///
    /// 参数 `life_cycle_holder` 提供当前 FlowBus 的隔离生命周期注册表；返回初始化
    /// 结果。对应 Java: `ScriptExecutor#init`。
    fn init(&self, life_cycle_holder: &LifeCycleHolder) -> LFResult<()> {
        self.life_cycle(life_cycle_holder);
        Ok(())
    }

    /// 通知当前 FlowBus 中的脚本引擎初始化生命周期。
    ///
    /// Java 传入异构引擎 `Object`；Rust 传递 `script_type` 的稳定 display name，
    /// 避免不可发送的引擎对象跨线程逃逸。参数 `life_cycle_holder` 是当前运行时的
    /// 隔离注册表。对应 Java: `ScriptExecutor#lifeCycle`。
    fn life_cycle(&self, life_cycle_holder: &LifeCycleHolder) {
        let language = self.script_type().get_display_name();
        for hook in life_cycle_holder.get_post_process_script_engine_init_life_cycle_list() {
            hook.post_process_after_script_engine_init(language);
        }
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

    /// 从执行器缓存中卸载指定节点脚本。
    ///
    /// 参数 `node_id` 只标识编译缓存，不删除 FlowBus 中的节点对象。该 Java
    /// 命名入口委托同一真实缓存操作。对应 Java: `ScriptExecutor#unLoad`。
    fn un_load(&self, node_id: &str) -> LFResult<()> {
        self.unload(node_id)
    }

    /// 返回当前执行器已经加载的全部节点 id。
    ///
    /// 对应 Java: `ScriptExecutor#getNodeIds`。
    fn node_ids(&self) -> LFResult<Vec<String>>;

    /// 返回当前执行器已经加载的全部节点 ID。
    ///
    /// 返回顺序遵循具体执行器的稳定排序规则。对应 Java:
    /// `ScriptExecutor#getNodeIds`。
    fn get_node_ids(&self) -> LFResult<Vec<String>> {
        self.node_ids()
    }

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

    /// 构建 Java `bindParam` 对应的公共脚本绑定表。
    ///
    /// 参数 `context` 提供本次执行的 Slot、节点和循环状态；返回表包含可由 serde
    /// 表达的上下文 Bean，以及 `_meta` 中的 slotIndex、当前 Chain、节点、标签、
    /// cmpData、循环数据、主流程请求和隐式子流程请求。`ScriptBeanProxy` 等 Rust
    /// trait object 由具体引擎通过受控调用桥单独绑定，不能伪装成 JSON。对应 Java:
    /// `ScriptExecutor#bindParam`。
    #[must_use]
    fn bind_param(&self, context: &CmpContext) -> Map<String, Value> {
        let mut bindings = Map::new();

        // Java 会逐个放入上下文 Bean；Rust 把不可变 JSON 和可并发写的
        // RwLock<Value> 都映射为 serde 快照。后者的 getter/setter 原地写回由
        // 具体脚本引擎的受控 Bean 桥处理。
        for entry in &context.inner.beans {
            if let Ok(value) = Arc::clone(entry.value()).downcast::<Value>() {
                bindings.insert(entry.key().clone(), (*value).clone());
            } else if let Ok(value) =
                Arc::clone(entry.value()).downcast::<std::sync::RwLock<Value>>()
                && let Ok(value) = value.read()
            {
                bindings.insert(entry.key().clone(), value.clone());
            }
        }

        let request_data = context
            .inner
            .input
            .lock()
            .map(|value| value.clone())
            .unwrap_or(Value::Null);
        let cmp_data = context
            .cmp_data_as::<Value>()
            .or_else(|| {
                context
                    .cmp_data()
                    .map(|value| Value::String(value.to_string()))
            })
            .unwrap_or(Value::Null);
        let mut meta = Map::new();
        meta.insert(
            "slotIndex".to_string(),
            context
                .slot_index()
                .map(|index| Value::Number(Number::from(index as u64)))
                .unwrap_or(Value::Null),
        );
        meta.insert(
            "currChainId".to_string(),
            Value::String(context.curr_chain_id().to_string()),
        );
        // Java 旧字段与 currChainId 同值，继续向旧脚本提供兼容元数据。
        meta.insert(
            "currChainName".to_string(),
            Value::String(context.curr_chain_id().to_string()),
        );
        meta.insert(
            "nodeId".to_string(),
            Value::String(context.node_id().to_string()),
        );
        meta.insert(
            "tag".to_string(),
            context
                .tag()
                .map(|tag| Value::String(tag.to_string()))
                .unwrap_or(Value::Null),
        );
        meta.insert("cmpData".to_string(), cmp_data.clone());
        meta.insert(
            "loopIndex".to_string(),
            context
                .loop_index()
                .map(|index| Value::Number(Number::from(index as u64)))
                .unwrap_or(Value::Null),
        );
        meta.insert(
            "loopObject".to_string(),
            context.frame.loop_object().cloned().unwrap_or(Value::Null),
        );
        meta.insert("requestData".to_string(), request_data.clone());
        if let Some(sub_request_data) = context.inner.get_chain_req_data(context.curr_chain_id()) {
            meta.insert("subRequestData".to_string(), sub_request_data);
        }

        // 这些 snake_case 名称是 Rust 脚本插件现有的受控快捷变量；它们与
        // Java `_meta` 同源，统一在此生成以防不同引擎逐渐漂移。
        bindings.insert("input".to_string(), request_data);
        bindings.insert(
            "node_id".to_string(),
            Value::String(context.node_id().to_string()),
        );
        bindings.insert(
            "tag".to_string(),
            context
                .tag()
                .map(|tag| Value::String(tag.to_string()))
                .unwrap_or(Value::Null),
        );
        bindings.insert("cmp_data".to_string(), cmp_data);
        bindings.insert(
            "loop_index".to_string(),
            context
                .loop_index()
                .map(|index| Value::Number(Number::from(index as u64)))
                .unwrap_or(Value::Null),
        );
        bindings.insert(
            "loop_object".to_string(),
            context.frame.loop_object().cloned().unwrap_or(Value::Null),
        );
        bindings.insert("_meta".to_string(), Value::Object(meta));
        bindings
    }

    /// 使用具体脚本引擎真实编译源代码，但不写入节点缓存。
    ///
    /// 参数 `script` 为原始脚本文本；成功表示编译器已生成可执行中间产物并立即
    /// 丢弃，失败返回引擎诊断。对应 Java: `ScriptExecutor#compile`。
    fn compile(&self, script: &str) -> LFResult<()>;

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

    /// 执行脚本节点访问判断。
    ///
    /// 参数 `wrap`、`context` 分别提供 Java 执行快照和 Rust 强类型上下文；默认
    /// 与 Java 一致允许执行。对应 Java: `ScriptExecutor#executeIsAccess`。
    fn execute_is_access(&self, wrap: &ScriptExecuteWrap, context: &CmpContext) -> bool {
        let _ = (wrap, context);
        true
    }

    /// 判断脚本节点异常后是否继续。
    ///
    /// 默认与 Java 一致返回 `false`。对应 Java:
    /// `ScriptExecutor#executeIsContinueOnError`。
    fn execute_is_continue_on_error(&self, wrap: &ScriptExecuteWrap, context: &CmpContext) -> bool {
        let _ = (wrap, context);
        false
    }

    /// 返回脚本执行器是否主动结束流程。
    ///
    /// 默认与 Java 一致返回 `false`；非 Java 脚本组件还会读取 Slot 的通用结束
    /// 标记。对应 Java: `ScriptExecutor#executeIsEnd`。
    fn execute_is_end(&self, wrap: &ScriptExecuteWrap, context: &CmpContext) -> bool {
        let _ = (wrap, context);
        false
    }

    /// 执行脚本组件前置切面。
    ///
    /// 参数 `wrap` 提供节点快照，`context` 提供真实 Slot。对应 Java:
    /// `ScriptExecutor#executeBeforeProcess`。
    fn execute_before_process(&self, wrap: &ScriptExecuteWrap, context: &CmpContext) {
        let _ = wrap;
        CmpAroundAspectHolder::load_cmp_around_aspect().before_process(context);
    }

    /// 执行脚本组件 finally 后置切面。
    ///
    /// 对应 Java: `ScriptExecutor#executeAfterProcess`。
    fn execute_after_process(&self, wrap: &ScriptExecuteWrap, context: &CmpContext) {
        let _ = wrap;
        CmpAroundAspectHolder::load_cmp_around_aspect().after_process(context);
    }

    /// 执行脚本组件成功切面。
    ///
    /// 对应 Java: `ScriptExecutor#executeOnSuccess`。
    fn execute_on_success(&self, wrap: &ScriptExecuteWrap, context: &CmpContext) {
        let _ = wrap;
        CmpAroundAspectHolder::load_cmp_around_aspect().on_success(context);
    }

    /// 执行脚本组件失败切面并保留原始错误。
    ///
    /// 参数 `error` 对应 Java 同名异常参数。对应 Java:
    /// `ScriptExecutor#executeOnError`。
    fn execute_on_error(
        &self,
        wrap: &ScriptExecuteWrap,
        context: &CmpContext,
        error: &LiteflowError,
    ) {
        let _ = wrap;
        CmpAroundAspectHolder::load_cmp_around_aspect().on_error(context, error);
    }

    /// 执行脚本组件回滚扩展。
    ///
    /// Java 基类默认无回滚副作用，具体执行器可覆盖；Rust 用成功的 `Result`
    /// 表达同一默认协议。对应 Java: `ScriptExecutor#executeRollback`。
    fn execute_rollback(&self, wrap: &ScriptExecuteWrap, context: &CmpContext) -> LFResult<()> {
        let _ = (wrap, context);
        Ok(())
    }
}
