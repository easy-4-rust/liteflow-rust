//! S2-B 验收测试：spi 包体系（5 接口 + 5 holder + 5 local + SpiFactoryCleaner）
//! 与 flow/id 包（RequestIdGenerator / DefaultRequestIdGenerator / IdGeneratorHolder）。
//!
//! 注意：holder 为进程级全局单例，凡涉及 register/clean 的测试用
//! `HOLDER_LOCK` 串行化，避免测试线程间相互干扰。

use std::sync::{Arc, Mutex};

use liteflow_core::spi::SpiFactoryCleaner;
use liteflow_core::spi::context_aware::ContextAware;
use liteflow_core::spi::holder::{
    CmpAroundAspectHolder, ContextAwareHolder, ContextCmpInitHolder,
    LiteflowComponentSupportHolder, PathContentParserHolder,
};
use liteflow_core::spi::liteflow_component_support::LiteflowComponentSupport;
use liteflow_core::spi::local::{
    LocalContextAware, LocalLiteflowComponentSupport, LocalPathContentParser,
};
use liteflow_core::spi::path_content_parser::PathContentParser;
use liteflow_core::spi::spi_priority::SpiPriority;
use liteflow_core::{
    DefaultRequestIdGenerator, IdGeneratorHolder, LiteflowConfig, LiteflowConfigGetter,
    RequestIdGenerator,
};

/// 串行化 holder 全局状态相关测试
static HOLDER_LOCK: Mutex<()> = Mutex::new(());

/// 对应 Java LocalContextAware：非容器环境不保存 Bean。
#[test]
fn local_context_aware_preserves_java_empty_container_semantics() {
    let aware = LocalContextAware::new();
    assert!(!aware.has_bean("svc"));

    // Java registerBean(String, Object) 原样返回对象，但不会把对象写入不存在的容器。
    let registered = aware.register_bean("svc", Arc::new(42_i32));
    assert_eq!(*registered.downcast::<i32>().unwrap(), 42);
    assert!(!aware.has_bean("svc"));
    assert!(aware.get_bean("svc").is_none());

    // Java registerOrGet 委托反射构造；本地空实现每次都产生新对象且不落库。
    let created = aware.register_or_get("other", &|| Arc::new("hello".to_string()));
    assert_eq!(created.downcast::<String>().unwrap().as_str(), "hello");
    assert!(!aware.has_bean("other"));
    assert!(aware.get_beans_of_type(None).is_none());
    assert!(!aware.has_bean_type("i32"));

    // 对应 priority()：本地实现优先级 2
    assert_eq!(aware.priority(), 2);
}

/// holder 未注册时回退各 Local 默认实现（对应 Java ServiceLoader 仅命中
/// local 实现时 list.get(0)）
#[test]
fn holders_fallback_to_local_defaults() {
    let _g = HOLDER_LOCK.lock().unwrap();
    SpiFactoryCleaner::clean();

    let aware = ContextAwareHolder::load_context_aware();
    assert_eq!(aware.priority(), 2);
    // 回退的 LocalContextAware 与 Java 一致，不提供本地 Bean 容器。
    let bean = aware.register_bean("k", Arc::new(1_i32));
    assert_eq!(*bean.downcast::<i32>().unwrap(), 1);
    assert!(!aware.has_bean("k"));

    assert_eq!(
        CmpAroundAspectHolder::load_cmp_around_aspect().priority(),
        2
    );
    assert_eq!(ContextCmpInitHolder::load_context_cmp_init().priority(), 2);
    assert_eq!(
        LiteflowComponentSupportHolder::load_liteflow_component_support().priority(),
        2
    );
    assert_eq!(
        PathContentParserHolder::load_path_content_parser().priority(),
        2
    );

    SpiFactoryCleaner::clean();
}

/// register 覆盖后，SpiFactoryCleaner.clean() 清理并回退 Local 默认实现
#[test]
fn spi_factory_cleaner_restores_local_fallback() {
    let _g = HOLDER_LOCK.lock().unwrap();

    /// 自定义高优先级实现（priority 数字越小优先级越高）
    struct MyAware;
    impl SpiPriority for MyAware {
        fn priority(&self) -> i32 {
            1
        }
    }
    impl ContextAware for MyAware {
        fn get_bean(&self, _name: &str) -> Option<liteflow_core::spi::Bean> {
            None
        }
        fn register_bean(
            &self,
            _name: &str,
            bean: liteflow_core::spi::Bean,
        ) -> liteflow_core::spi::Bean {
            bean
        }
        fn has_bean(&self, _name: &str) -> bool {
            false
        }
        fn register_or_get(
            &self,
            _name: &str,
            factory: &dyn Fn() -> liteflow_core::spi::Bean,
        ) -> liteflow_core::spi::Bean {
            factory()
        }
    }

    ContextAwareHolder::register(Arc::new(MyAware));
    assert_eq!(ContextAwareHolder::load_context_aware().priority(), 1);

    // 对应 SpiFactoryCleaner.clean()：清理后回退 Local（priority=2）
    SpiFactoryCleaner::clean();
    assert_eq!(ContextAwareHolder::load_context_aware().priority(), 2);

    SpiFactoryCleaner::clean();
}

/// 对应 IdGeneratorHolder：默认生成非空、格式为 fastSimpleUUID（32 位十六进制）
/// 且两次生成不同；register 自定义生成器后生效，clean 回退默认
#[test]
fn id_generator_holder_default_and_custom() {
    let _g = HOLDER_LOCK.lock().unwrap();
    IdGeneratorHolder::clean();

    let id1 = IdGeneratorHolder::generate();
    let id2 = IdGeneratorHolder::generate();
    assert!(!id1.is_empty());
    assert_eq!(id1.len(), 32);
    assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(id1, id2);

    // DefaultRequestIdGenerator 直接调用（对应 IdUtil.fastSimpleUUID 语义）
    let generator = DefaultRequestIdGenerator::new();
    let a = generator.generate();
    let b = generator.generate();
    assert_eq!(a.len(), 32);
    assert_ne!(a, b);

    // 对应 setRequestIdGenerator：注册自定义生成器
    struct Fixed;
    impl RequestIdGenerator for Fixed {
        fn generate(&self) -> String {
            "fixed-id".to_string()
        }
    }
    IdGeneratorHolder::register(Arc::new(Fixed));
    assert_eq!(IdGeneratorHolder::generate(), "fixed-id");
    assert!(std::ptr::eq(
        IdGeneratorHolder::get_instance(),
        IdGeneratorHolder::get_instance()
    ));
    assert_eq!(
        IdGeneratorHolder::get_request_id_generator()
            .expect("自定义生成器应保存在共享 holder")
            .generate(),
        "fixed-id"
    );

    // Rust 以显式类名注册表替代 Java 反射，首次 generate 必须按真实配置懒初始化。
    let mut config = LiteflowConfig::default();
    config.set_request_id_generator_class("test.FixedRequestIdGenerator");
    LiteflowConfigGetter::set_liteflow_config(config);
    IdGeneratorHolder::register_named("test.FixedRequestIdGenerator", Arc::new(Fixed));
    IdGeneratorHolder::clean();
    assert_eq!(IdGeneratorHolder::generate(), "fixed-id");
    assert_eq!(
        IdGeneratorHolder::get_request_id_generator()
            .expect("配置类名应解析为已注册生成器")
            .generate(),
        "fixed-id"
    );

    // clean 后回退默认生成器
    LiteflowConfigGetter::clean();
    IdGeneratorHolder::clean();
    let id3 = IdGeneratorHolder::generate();
    assert_eq!(id3.len(), 32);
    assert_ne!(id3, "fixed-id");
}

/// 对应 LocalPathContentParser：file:// 前缀与裸路径读文件；
/// 空路径列表报 ConfigErrorException；classpath: 运行期不支持
#[test]
fn local_path_content_parser_file_and_classpath() {
    let parser = LocalPathContentParser::new();
    assert_eq!(parser.priority(), 2);

    let dir = std::env::temp_dir().join("s2_spi_path_parser");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("rule.json");
    std::fs::write(&file, r#"{"chain":"c1"}"#).unwrap();
    let abs = file.to_string_lossy().into_owned();

    // 裸绝对路径
    let contents = parser.parse_content(&[abs.clone()]).unwrap();
    assert_eq!(contents, vec![r#"{"chain":"c1"}"#.to_string()]);

    // file:// 前缀
    let contents = parser.parse_content(&[format!("file://{abs}")]).unwrap();
    assert_eq!(contents.len(), 1);

    // file: 前缀
    let contents = parser.parse_content(&[format!("file:{abs}")]).unwrap();
    assert_eq!(contents.len(), 1);

    // getFileAbsolutePath：返回存在的文件、静默跳过不存在的路径
    let paths = parser
        .get_file_absolute_path(&[abs.clone(), "/nonexistent/rule.json".to_string()])
        .unwrap();
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("rule.json"));

    // 空列表 → ConfigErrorException("rule source must not be null")
    let err = parser.parse_content(&[]).unwrap_err();
    assert!(err.to_string().contains("rule source must not be null"));

    // classpath: 运行期不支持 → ConfigErrorException
    let err = parser
        .parse_content(&["classpath:rule.json".to_string()])
        .unwrap_err();
    assert!(err.to_string().contains("classpath:"));

    std::fs::remove_dir_all(&dir).ok();
}

/// 对应 PathMatchUtil：绝对路径的 `*`/`**` 展开、稳定排序与去重。
#[test]
fn local_path_content_parser_expands_absolute_ant_patterns() {
    let parser = LocalPathContentParser::new();
    let directory = tempfile::tempdir().unwrap();
    let nested = directory.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(directory.path().join("a.json"), r#"{"id":"a"}"#).unwrap();
    std::fs::write(nested.join("b.json"), r#"{"id":"b"}"#).unwrap();
    std::fs::write(nested.join("ignored.xml"), "<flow/>").unwrap();

    let recursive = format!("{}/**/*.json", directory.path().display());
    let direct = format!("{}/*.json", directory.path().display());
    let paths = parser
        .get_file_absolute_path(&[recursive.clone(), direct])
        .unwrap();

    assert_eq!(paths.len(), 2);
    assert!(paths[0].ends_with("a.json"));
    assert!(paths[1].ends_with("nested/b.json"));
    let contents = parser.parse_content(&[recursive]).unwrap();
    assert_eq!(contents.len(), 2);
    assert!(contents.iter().any(|content| content.contains("\"a\"")));
    assert!(contents.iter().any(|content| content.contains("\"b\"")));
}

/// 对应 LocalLiteflowComponentSupport：返回组件自身 name()
#[test]
fn local_liteflow_component_support_returns_cmp_name() {
    use async_trait::async_trait;
    use liteflow_core::core::NodeComponent;
    use liteflow_core::exception::LiteflowError;
    use liteflow_core::slot::CmpContext;
    use serde_json::Value;

    struct CmpA;
    #[async_trait]
    impl NodeComponent for CmpA {
        async fn process(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
            Ok(Value::Null)
        }
        fn name(&self) -> &str {
            "cmp_a"
        }
    }

    let support = LocalLiteflowComponentSupport::new();
    let cmp = CmpA;
    assert_eq!(support.get_cmp_name(&cmp), Some("cmp_a".to_string()));
}
