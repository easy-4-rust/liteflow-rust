//! 按规则地址选择解析器。

use std::sync::Arc;

use crate::enums::FlowParserTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::factory::{ClassParserFactory, FlowParserFactory, LocalParserFactory};

/// 在本地文件解析器与显式注册的自定义解析器之间完成选择。
///
/// Java 使用正则与 `Class.forName`；Rust 保留相同前缀/后缀协议，并把类反射
/// 替换为 `ClassParserFactory` 的类型安全注册表。
///
/// 对应 Java: `com.yomahub.liteflow.parser.factory.FlowParserProvider`。
#[derive(Clone)]
pub struct FlowParserProvider {
    local_factory: LocalParserFactory,
    class_factory: ClassParserFactory,
}

impl FlowParserProvider {
    /// 使用目标流程总线创建解析器提供者。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            local_factory: LocalParserFactory::new(bus.clone()),
            class_factory: ClassParserFactory::new(bus),
        }
    }

    /// 注册自定义解析器内容源。
    ///
    /// 参数与 `ClassParserFactory#register` 一致；`content_provider` 对应
    /// Java 自定义 Parser 的 `parseCustom()`。
    pub fn register_class_parser(
        &self,
        class_name: impl Into<String>,
        parser_type: FlowParserTypeEnum,
        content_provider: Arc<dyn Fn() -> LFResult<String> + Send + Sync>,
    ) {
        self.class_factory
            .register(class_name, parser_type, content_provider);
    }

    /// 根据配置地址返回对应解析器。
    ///
    /// 支持 `.xml/.json/.yml` 与 `.el.xml/.el.json/.el.yml` 本地路径，
    /// 以及 `xml:/json:/yml:/el_xml:/el_json:/el_yml:` 自定义解析器前缀。
    /// 对应 Java: `FlowParserProvider#lookup`。
    pub fn lookup(&self, path: &str) -> LFResult<Box<dyn FlowParser>> {
        if let Some(parser_type) = local_parser_type(path) {
            return create_with_factory(&self.local_factory, path, parser_type);
        }

        let (prefix_type, class_name) = split_class_path(path)?;
        if class_name.is_empty() {
            return Err(unsupported_path(path));
        }
        match prefix_type {
            Some(parser_type) => self.class_factory.create_for_type(class_name, parser_type),
            None => self.class_factory.create_registered(class_name),
        }
    }
}

fn create_with_factory(
    factory: &dyn FlowParserFactory,
    path: &str,
    parser_type: FlowParserTypeEnum,
) -> LFResult<Box<dyn FlowParser>> {
    match parser_type {
        FlowParserTypeEnum::TypeXml | FlowParserTypeEnum::TypeElXml => {
            factory.create_xml_el_parser(path)
        }
        FlowParserTypeEnum::TypeJson | FlowParserTypeEnum::TypeElJson => {
            factory.create_json_el_parser(path)
        }
        FlowParserTypeEnum::TypeYml | FlowParserTypeEnum::TypeElYml => {
            factory.create_yml_el_parser(path)
        }
    }
}

fn local_parser_type(path: &str) -> Option<FlowParserTypeEnum> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".el.xml") {
        Some(FlowParserTypeEnum::TypeElXml)
    } else if lower.ends_with(".el.json") {
        Some(FlowParserTypeEnum::TypeElJson)
    } else if lower.ends_with(".el.yml") {
        Some(FlowParserTypeEnum::TypeElYml)
    } else if lower.ends_with(".xml") {
        Some(FlowParserTypeEnum::TypeXml)
    } else if lower.ends_with(".json") {
        Some(FlowParserTypeEnum::TypeJson)
    } else if lower.ends_with(".yml") {
        Some(FlowParserTypeEnum::TypeYml)
    } else {
        None
    }
}

fn split_class_path(path: &str) -> LFResult<(Option<FlowParserTypeEnum>, &str)> {
    let Some((prefix, class_name)) = path.split_once(':') else {
        return Ok((None, path));
    };
    let parser_type =
        FlowParserTypeEnum::get_enum_by_type(prefix).ok_or_else(|| unsupported_path(path))?;
    Ok((Some(parser_type), class_name))
}

fn unsupported_path(path: &str) -> LiteflowError {
    LiteflowError::ErrorSupportPath(format!("can't support the format {path}"))
}
