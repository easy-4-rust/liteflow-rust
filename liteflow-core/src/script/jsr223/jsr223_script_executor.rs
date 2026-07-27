//! 对应 Java: com.yomahub.liteflow.script.jsr223.JSR223ScriptExecutor

use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;

use crate::core::NodeComponent;
use crate::exception::LFResult;
use crate::script::exception::ScriptLoadException;
use crate::script::{ScriptExecuteWrap, ScriptExecutorFactory, ScriptKind, build_rhai_component};
use crate::slot::CmpContext;

/// 以 Rust 语言插件工厂替代 JVM JSR223，并缓存每个节点的已编译组件。
pub struct JSR223ScriptExecutor {
    language: String,
    kind: ScriptKind,
    compiled_script_map: DashMap<String, Arc<dyn NodeComponent>>,
}

impl JSR223ScriptExecutor {
    /// 创建指定语言与节点类别的执行器。
    #[must_use]
    pub fn new(language: impl Into<String>, kind: ScriptKind) -> Self {
        Self {
            language: language.into(),
            kind,
            compiled_script_map: DashMap::new(),
        }
    }

    /// 初始化语言引擎并验证 SPI 是否可用。
    pub fn init(&self) -> LFResult<()> {
        if self.language == "rhai" || ScriptExecutorFactory::contains(&self.language) {
            Ok(())
        } else {
            Err(crate::script::exception::ScriptSpiException::new(format!(
                "unsupported script language: {}",
                self.language
            ))
            .into())
        }
    }

    /// 编译并按节点 id 缓存脚本。对应 Java `load`。
    pub fn load(&self, node_id: &str, script: &str) -> LFResult<()> {
        let component = self.compile(node_id, script)?;
        self.compiled_script_map
            .insert(node_id.to_string(), component);
        Ok(())
    }

    /// 编译脚本但不修改缓存。对应 Java `compile` 与 `convertScript`。
    pub fn compile(&self, node_id: &str, script: &str) -> LFResult<Arc<dyn NodeComponent>> {
        if self.language == "rhai" {
            build_rhai_component(node_id, self.kind, script)
        } else {
            ScriptExecutorFactory::build(&self.language, node_id, self.kind, script)
        }
        .map_err(|error| {
            ScriptLoadException::new(node_id, format!("load script failed: {error}")).into()
        })
    }

    /// 卸载节点脚本。对应 Java `unLoad`。
    pub fn unload(&self, node_id: &str) {
        self.compiled_script_map.remove(node_id);
    }

    /// 卸载指定节点的已编译脚本。
    ///
    /// # 参数
    /// - `node_id`: 需要从编译缓存移除的节点 ID。
    ///
    /// 对应 Java: `JSR223ScriptExecutor#unLoad`。
    pub fn un_load(&self, node_id: &str) {
        self.unload(node_id);
    }

    /// 返回排序后的已加载节点 id。对应 Java `getNodeIds`。
    #[must_use]
    pub fn node_ids(&self) -> Vec<String> {
        let mut node_ids = self
            .compiled_script_map
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        node_ids.sort();
        node_ids
    }

    /// 返回已经加载脚本的节点 ID。
    ///
    /// # 返回
    /// 为保证 Rust 调用结果稳定，返回按字典序排序的拥有型列表。
    ///
    /// 对应 Java: `JSR223ScriptExecutor#getNodeIds`。
    #[must_use]
    pub fn get_node_ids(&self) -> Vec<String> {
        self.node_ids()
    }

    /// 执行已加载脚本。
    ///
    /// 先用 `ScriptExecuteWrap` 校验节点缓存，再把真实 `CmpContext` 交给组件，
    /// 保证共享数据、循环帧与请求数据仍沿主执行链传播。
    pub async fn execute(
        &self,
        execute_wrap: &ScriptExecuteWrap,
        context: &CmpContext,
    ) -> LFResult<Value> {
        let component = self
            .compiled_script_map
            .get(execute_wrap.node_id())
            .map(|entry| entry.clone())
            .ok_or_else(|| {
                ScriptLoadException::new(
                    execute_wrap.node_id(),
                    format!(
                        "script node[{}] has not been loaded",
                        execute_wrap.node_id()
                    ),
                )
            })?;
        component.process(context).await
    }

    /// 执行已加载的脚本。
    ///
    /// # 参数
    /// - `execute_wrap`: Java `ScriptExecuteWrap` 对应的执行元数据快照。
    /// - `context`: Rust 异步执行链持有的真实组件上下文。
    ///
    /// # 返回
    /// 脚本执行结果；节点未加载或脚本失败时返回对应错误。
    ///
    /// Java 通过线程本地 Slot 恢复上下文；Rust 显式传入 `CmpContext`，避免跨
    /// `await` 的线程局部状态错配。对应 Java:
    /// `JSR223ScriptExecutor#executeScript`。
    pub async fn execute_script(
        &self,
        execute_wrap: &ScriptExecuteWrap,
        context: &CmpContext,
    ) -> LFResult<Value> {
        self.execute(execute_wrap, context).await
    }

    /// 清空已编译脚本缓存。对应 Java `cleanScriptCache`。
    pub fn clean_script_cache(&self) {
        self.compiled_script_map.clear();
    }

    /// 清空全部已编译脚本缓存。
    ///
    /// 对应 Java: `JSR223ScriptExecutor#cleanCache`。
    pub fn clean_cache(&self) {
        self.clean_script_cache();
    }
}
