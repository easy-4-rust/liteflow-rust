use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

/// 单次组件执行统计记录。
///
/// Java 对象以组件类名、耗时、内存占用和记录时间描述一次执行，并按记录时间
/// 从新到旧排序。Rust 同时保留累计成功/失败等公开字段，作为既有诊断 API 的
/// 报表快照扩展；单次记录由 `new` 创建时这些累计字段取默认值。
///
/// 对应 Java: `com.yomahub.liteflow.monitor.CompStatistics`。
#[derive(Debug, Clone)]
pub struct CompStatistics {
    component_clazz_name: String,
    time_spent: u64,
    memory_spent: u64,
    record_time: u64,
    /// Rust 报表兼容字段：组件或节点 id。
    pub node_id: String,
    /// Rust 报表兼容字段：累计执行次数。
    pub total: u64,
    /// Rust 报表兼容字段：累计成功次数。
    pub success: u64,
    /// Rust 报表兼容字段：累计失败次数。
    pub fail: u64,
    /// Rust 报表兼容字段：有界样本平均耗时。
    pub avg_time_ms: u64,
    /// Rust 报表兼容字段：有界样本最大耗时。
    pub max_time_ms: u64,
}

impl CompStatistics {
    /// 创建一条组件执行统计并记录当前毫秒时间戳。
    ///
    /// 参数 `component_clazz_name` 为组件展示名，`time_spent` 为毫秒耗时。
    /// 对应 Java: `CompStatistics#CompStatistics(String,long)`。
    #[must_use]
    pub fn new(component_clazz_name: impl Into<String>, time_spent: u64) -> Self {
        let component_clazz_name = component_clazz_name.into();
        Self {
            node_id: component_clazz_name.clone(),
            component_clazz_name,
            time_spent,
            memory_spent: 0,
            record_time: current_time_millis(),
            total: 1,
            success: 0,
            fail: 0,
            avg_time_ms: time_spent,
            max_time_ms: time_spent,
        }
    }

    /// 根据 MonitorBus 的累计计数创建报表快照。
    ///
    /// 参数依次为组件名、总次数、成功数、失败数、平均耗时和最大耗时；返回对象
    /// 保留 Java CompStatistics 字段，并填充 Rust 累计诊断扩展字段。
    pub(crate) fn aggregate(
        component_clazz_name: String,
        total: u64,
        success: u64,
        fail: u64,
        avg_time_ms: u64,
        max_time_ms: u64,
    ) -> Self {
        let mut statistics = Self::new(component_clazz_name, avg_time_ms);
        statistics.total = total;
        statistics.success = success;
        statistics.fail = fail;
        statistics.max_time_ms = max_time_ms;
        statistics
    }

    /// 返回组件类名。对应 Java: `CompStatistics#getComponentClazzName`。
    #[must_use]
    pub fn get_component_clazz_name(&self) -> &str {
        &self.component_clazz_name
    }

    /// 返回组件类名。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_component_clazz_name`。
    #[must_use]
    pub fn component_clazz_name(&self) -> &str {
        self.get_component_clazz_name()
    }

    /// 设置组件类名。对应 Java: `CompStatistics#setComponentClazzName`。
    pub fn set_component_clazz_name(&mut self, component_clazz_name: impl Into<String>) {
        self.component_clazz_name = component_clazz_name.into();
        self.node_id = self.component_clazz_name.clone();
    }

    /// 返回单次执行耗时（毫秒）。对应 Java: `CompStatistics#getTimeSpent`。
    #[must_use]
    pub fn get_time_spent(&self) -> u64 {
        self.time_spent
    }

    /// 返回单次执行耗时（毫秒）。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_time_spent`。
    #[must_use]
    pub fn time_spent(&self) -> u64 {
        self.get_time_spent()
    }

    /// 设置单次执行耗时（毫秒）。对应 Java: `CompStatistics#setTimeSpent`。
    pub fn set_time_spent(&mut self, time_spent: u64) {
        self.time_spent = time_spent;
        self.avg_time_ms = time_spent;
        self.max_time_ms = time_spent;
    }

    /// 返回内存占用统计。对应 Java: `CompStatistics#getMemorySpent`。
    #[must_use]
    pub fn get_memory_spent(&self) -> u64 {
        self.memory_spent
    }

    /// 返回内存占用统计。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_memory_spent`。
    #[must_use]
    pub fn memory_spent(&self) -> u64 {
        self.get_memory_spent()
    }

    /// 设置内存占用统计。对应 Java: `CompStatistics#setMemorySpent`。
    pub fn set_memory_spent(&mut self, memory_spent: u64) {
        self.memory_spent = memory_spent;
    }

    /// 返回记录创建时间的毫秒时间戳。对应 Java: `CompStatistics#getRecordTime`。
    #[must_use]
    pub fn get_record_time(&self) -> u64 {
        self.record_time
    }

    /// 返回记录创建时间的毫秒时间戳。
    ///
    /// Rust 历史便捷入口；Java 对等名称请使用 `get_record_time`。
    #[must_use]
    pub fn record_time(&self) -> u64 {
        self.get_record_time()
    }

    /// 比较两条统计记录，新记录排在旧记录之前。
    ///
    /// 参数 `other` 对应 Java 同名参数；`None` 对齐 Java 的 null，当前对象排在
    /// null 之后并返回 `Ordering::Greater`。对应 Java:
    /// `CompStatistics#compareTo`。
    #[must_use]
    pub fn compare_to(&self, other: Option<&Self>) -> Ordering {
        let Some(other) = other else {
            return Ordering::Greater;
        };
        other.get_record_time().cmp(&self.record_time)
    }
}

impl PartialEq for CompStatistics {
    fn eq(&self, other: &Self) -> bool {
        self.record_time == other.record_time
    }
}

impl Eq for CompStatistics {}

impl PartialOrd for CompStatistics {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompStatistics {
    /// 按记录时间从新到旧排序。对应 Java: `CompStatistics#compareTo`。
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare_to(Some(other))
    }
}

impl std::fmt::Display for CompStatistics {
    /// 输出累计报表摘要。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: total={}, success={}, fail={}, avg={}ms, max={}ms",
            self.node_id, self.total, self.success, self.fail, self.avg_time_ms, self.max_time_ms
        )
    }
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
