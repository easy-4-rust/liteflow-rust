//! 对应 flow.entity.CmpStep。

use crate::enums::CmpStepTypeEnum;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct CmpStep {
    pub node_id: String,
    pub node_name: String,
    pub tag: Option<String>,
    pub step_type: CmpStepTypeEnum,
    pub start: Instant,
    pub end: Option<Instant>,
    pub time_spent: Option<Duration>,
    pub success: bool,
    pub exception: Option<String>,
    pub rollback_time_spent: Option<Duration>,
    pub thread_name: String,
}

impl CmpStep {
    pub fn new(node_id: impl Into<String>, step_type: CmpStepTypeEnum) -> Self {
        Self {
            node_id: node_id.into(),
            node_name: String::new(),
            tag: None,
            step_type,
            start: Instant::now(),
            end: None,
            time_spent: None,
            success: false,
            exception: None,
            rollback_time_spent: None,
            thread_name: std::thread::current().name().unwrap_or("unnamed").to_string(),
        }
    }

    pub fn finish(&mut self, success: bool, exception: Option<String>) {
        self.end = Some(Instant::now());
        self.time_spent = Some(self.end.unwrap().saturating_duration_since(self.start));
        self.success = success;
        self.exception = exception;
    }

    /// buildTimeSpent（毫秒）
    pub fn time_spent_ms(&self) -> u128 {
        self.time_spent.map(|d| d.as_millis()).unwrap_or(0)
    }
}
