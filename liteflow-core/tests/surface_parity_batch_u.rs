//! Java 表面语义回归批次 U：枚举/SPI 默认方法/工具类补齐。
//!
//! 覆盖对象与 Java 对应关系：
//! - `TimeUnit#to_duration`：Java `TimeUnit` 各单位的 `toMillis/toSeconds` 转换
//! - `CmpStepTypeEnum#as_str`：Java 枚举 `name()` 语义
//! - `ParallelStrategyEnum#getStrategyType/getDescription/getClazz`
//! - `MonitorBus#record/report`：Java `StatEntry` 平均耗时聚合
//! - `ParallelSupplier`：Java `Supplier<WhenFutureObj>` 的 Rust async trait 形态
//! - `ContextAware`/`DeclComponentParser` 默认方法：Java SPI 默认行为
//! - `ICmpAroundAspect` 默认回调：Java default 方法空实现
//! - `LOGOPrinter#logo/print`

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use liteflow_core::aop::ICmpAroundAspect;
use liteflow_core::core::proxy::DeclWarpBean;
use liteflow_core::core::DeclComponent;
use liteflow_core::enums::{CmpStepTypeEnum, ParallelStrategyEnum};
use liteflow_core::flow::parallel::{ParallelSupplier, WhenFutureObj};
use liteflow_core::monitor::MonitorBus;
use liteflow_core::property::TimeUnit;
use liteflow_core::slot::{CmpContext, Frame};
use liteflow_core::spi::{Bean, ContextAware, DeclComponentParser, SpiPriority};
use liteflow_core::util::LOGOPrinter;
use liteflow_core::{CmpStep, CmpStepType, LiteflowError, NodeRef, NodeTypeEnum, Slot};
use serde_json::{Value, json};

/// Java `TimeUnit` 七个单位的转换语义：`to_duration` 数值必须与 Java
/// `TimeUnit#toNanos/toMicros/toMillis/toSeconds` 对等。
#[test]
fn time_unit_all_java_units_convert_to_expected_durations() {
    assert_eq!(TimeUnit::Nanoseconds.to_duration(2), Duration::from_nanos(2));
    assert_eq!(
        TimeUnit::Microseconds.to_duration(3),
        Duration::from_micros(3)
    );
    assert_eq!(
        TimeUnit::Milliseconds.to_duration(4),
        Duration::from_millis(4)
    );
    assert_eq!(TimeUnit::Seconds.to_duration(5), Duration::from_secs(5));
    assert_eq!(TimeUnit::Minutes.to_duration(2), Duration::from_secs(120));
    assert_eq!(TimeUnit::Hours.to_duration(2), Duration::from_secs(7200));
    assert_eq!(
        TimeUnit::Days.to_duration(2),
        Duration::from_secs(2 * 24 * 60 * 60)
    );
    // Java 默认单位是毫秒；配置反序列化使用 SCREAMING_SNAKE_CASE 常量名。
    assert_eq!(TimeUnit::default(), TimeUnit::Milliseconds);
    let parsed: TimeUnit = serde_json::from_str(r#""MINUTES""#).unwrap();
    assert_eq!(parsed, TimeUnit::Minutes);
}

/// `CmpStepTypeEnum#name()`：START/END/SINGLE 常量名与 Java 枚举一致；
/// 兼容别名 `CmpStepType` 指向同一类型。
#[test]
fn cmp_step_type_enum_exposes_java_constant_names() {
    assert_eq!(CmpStepTypeEnum::Start.as_str(), "START");
    assert_eq!(CmpStepTypeEnum::End.as_str(), "END");
    assert_eq!(CmpStepTypeEnum::Single.as_str(), "SINGLE");
    let legacy: CmpStepType = CmpStepTypeEnum::Single;
    assert_eq!(legacy, CmpStepTypeEnum::Single);
    let step = CmpStep::new("a", "节点", CmpStepTypeEnum::Start);
    assert_eq!(step.get_step_type().as_str(), "START");
}

/// `ParallelStrategyEnum` 三组访问器：类型串、中文说明与执行器类名逐项保留
/// Java 枚举构造参数。
#[test]
fn parallel_strategy_enum_java_accessors_are_complete() {
    for (strategy, strategy_type, description, clazz) in [
        (
            ParallelStrategyEnum::Any,
            "anyOf",
            "完成任一任务",
            "AnyOfParallelExecutor",
        ),
        (
            ParallelStrategyEnum::All,
            "allOf",
            "完成全部任务",
            "AllOfParallelExecutor",
        ),
        (
            ParallelStrategyEnum::Specify,
            "must",
            "完成指定 ID 任务",
            "SpecifyParallelExecutor",
        ),
        (
            ParallelStrategyEnum::Percentage,
            "percentageOf",
            "完整指定阈值任务",
            "PercentageOfParallelExecutor",
        ),
    ] {
        assert_eq!(strategy.get_strategy_type(), strategy_type);
        assert_eq!(strategy.get_description(), description);
        assert_eq!(strategy.get_clazz(), clazz);
    }
}

/// `MonitorBus#record/report`：Java 统计项按组件聚合样本数、成功/失败与总耗时，
/// 平均耗时 = 总耗时 / 样本数。
#[test]
fn monitor_bus_record_reports_java_stat_entry_aggregation() {
    let bus = MonitorBus::new();
    bus.record("cmp_a", Duration::from_millis(100), true);
    bus.record("cmp_a", Duration::from_millis(200), true);
    bus.record("cmp_b", Duration::from_millis(50), false);

    let statistics = bus.report();
    assert_eq!(statistics.len(), 2);
    let a = statistics
        .iter()
        .find(|stat| stat.node_id == "cmp_a")
        .unwrap();
    assert_eq!(a.total, 2);
    assert_eq!(a.success, 2);
    assert_eq!(a.fail, 0);
    assert_eq!(a.avg_time_ms, 150);
    let b = statistics
        .iter()
        .find(|stat| stat.node_id == "cmp_b")
        .unwrap();
    assert_eq!(b.total, 1);
    assert_eq!(b.success, 0);
    assert_eq!(b.fail, 1);
    assert_eq!(b.avg_time_ms, 50);
    assert!(bus.print_statistics().contains("cmp_a"));
}

/// `ParallelSupplier`（Java `Supplier<WhenFutureObj>`）：闭包实现通过
/// `get()` 返回真实结果载体。
#[tokio::test]
async fn parallel_supplier_closure_impl_yields_when_future_object() {
    let supplier: &dyn ParallelSupplier = &(|| async { WhenFutureObj::success("executor-a") });
    let outcome = supplier.get().await;
    assert!(outcome.is_success());
    assert_eq!(outcome.get_executor_id(), "executor-a");

    let failing: &dyn ParallelSupplier =
        &(|| async { WhenFutureObj::fail("executor-b", LiteflowError::NodeNotFound("x".into())) });
    let failed = failing.get().await;
    assert!(!failed.is_success());
    assert!(failed.get_ex().is_some());
}

/// `ContextAware` 默认方法：无容器实现的 `getBeansOfType`/`hasBean(Class)`/
/// `registerDeclWrapBean` 按 Java SPI 默认行为返回 null/false。
#[test]
fn context_aware_default_methods_match_java_spi_fallbacks() {
    let aware = MinimalContextAware;
    assert!(aware.get_beans_of_type(None).is_none());
    assert!(aware.get_beans_of_type(Some("any")).is_none());
    assert!(!aware.has_bean_type("any"));
    let declaration = DeclWarpBean::new(
        "decl",
        "声明",
        NodeTypeEnum::Common,
        Arc::new(PassThroughDecl),
        "tests::PassThroughDecl",
        Vec::new(),
    );
    assert!(aware.register_decl_wrap_bean("decl", declaration).is_none());
}

/// `DeclComponentParser#parseDeclBean(Class,String,String)` 默认实现：先写
/// nodeId/nodeName 再转发到单参解析。
#[test]
fn decl_component_parser_default_identity_prefixes_and_forwards() {
    let parser = EchoDeclComponentParser;
    let declaration = DeclWarpBean::new(
        "original",
        "原名",
        NodeTypeEnum::Common,
        Arc::new(PassThroughDecl),
        "tests::PassThroughDecl",
        Vec::new(),
    );
    let parsed = parser
        .parse_decl_bean_with_identity(declaration, "renamed", "新名")
        .unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].get_node_id(), "renamed");
    assert_eq!(parsed[0].get_node_name(), "新名");
}

/// `ICmpAroundAspect` 四个默认回调：未覆盖默认方法时按 Java default 语义
/// 保持空操作且不改变上下文。
#[tokio::test]
async fn i_cmp_around_aspect_default_callbacks_are_inert() {
    let slot = Arc::new(Slot::new("RID-AOP".to_string(), "main", Value::Null));
    let context = CmpContext {
        inner: slot.clone(),
        node: NodeRef::new("a"),
        frame: Frame::root(),
    };
    let aspect = NoopAspect;
    aspect.before_process(&context);
    aspect.on_success(&context);
    aspect.after_process(&context);
    aspect.on_error(&context, &LiteflowError::NodeNotFound("a".to_string()));
    assert_eq!(slot.get_request_id(), "RID-AOP");
}

/// `LOGOPrinter#logo`：版本横幅包含版本号、标语与官网地址；
/// `print` 走日志门面不 panic。
#[test]
fn logo_printer_renders_version_banner_with_website() {
    let logo = LOGOPrinter::logo();
    assert!(logo.contains("Version:"));
    assert!(logo.contains(env!("CARGO_PKG_VERSION")));
    assert!(logo.contains("Make your code amazing."));
    assert!(logo.contains("https://liteflow.cc"));
    LOGOPrinter::print();
}

/// 最小 `ContextAware` 实现：只实现必需方法，其余走默认。
struct MinimalContextAware;

impl SpiPriority for MinimalContextAware {
    fn priority(&self) -> i32 {
        1
    }
}

impl ContextAware for MinimalContextAware {
    fn get_bean(&self, _bean_name: &str) -> Option<Bean> {
        None
    }

    fn register_bean(&self, bean_name: &str, bean: Bean) -> Bean {
        let _ = bean_name;
        bean
    }

    fn has_bean(&self, _bean_name: &str) -> bool {
        false
    }

    fn register_or_get(&self, bean_name: &str, factory: &dyn Fn() -> Bean) -> Bean {
        let _ = bean_name;
        factory()
    }
}

/// 最小 `DeclComponentParser`：只实现单参解析并原样返回。
struct EchoDeclComponentParser;

impl SpiPriority for EchoDeclComponentParser {
    fn priority(&self) -> i32 {
        1
    }
}

impl DeclComponentParser for EchoDeclComponentParser {
    fn parse_decl_bean(
        &self,
        decl_warp_bean: DeclWarpBean,
    ) -> liteflow_core::exception::LFResult<Vec<DeclWarpBean>> {
        Ok(vec![decl_warp_bean])
    }
}

/// 不覆盖任何回调的切面实现。
struct NoopAspect;

impl ICmpAroundAspect for NoopAspect {}

/// 声明式组件最小实现（承接 `DeclWarpBean` 的 `Arc<dyn DeclComponent>`）。
struct PassThroughDecl;

#[async_trait]
impl DeclComponent for PassThroughDecl {
    async fn call(&self, _method: &str, _context: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(json!({"ok": true}))
    }
}
