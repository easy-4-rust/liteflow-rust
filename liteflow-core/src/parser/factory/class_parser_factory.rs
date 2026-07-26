//! 自定义内容源解析器工厂。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::enums::FlowParserTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::flow_bus::FlowBus;
use crate::parser::base::FlowParser;
use crate::parser::el::{ClassJsonFlowElParser, ClassXmlFlowElParser, ClassYmlFlowElParser};
use crate::parser::factory::FlowParserFactory;

type ContentProvider = Arc<dyn Fn() -> LFResult<String> + Send + Sync>;

/// 用显式注册表替代 Java `Class.forName` 与容器 `registerBean`。
///
/// 注册项保留类名、格式与内容提供器；创建解析器时严格校验请求格式，避免
/// 把 JSON 内容交给 XML/YML 解析器。
///
/// 对应 Java: `com.yomahub.liteflow.parser.factory.ClassParserFactory`。
#[derive(Clone)]
pub struct ClassParserFactory {
    bus: FlowBus,
    registrations: Arc<RwLock<HashMap<String, (FlowParserTypeEnum, ContentProvider)>>>,
}

impl ClassParserFactory {
    /// 使用目标流程总线创建自定义解析器工厂。
    #[must_use]
    pub fn new(bus: FlowBus) -> Self {
        Self {
            bus,
            registrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册一个自定义内容源。
    ///
    /// 参数 `class_name` 对应 Java 自定义 Parser 类全名；`parser_type`
    /// 表示内容格式；`content_provider` 对应其 `parseCustom()`。
    pub fn register(
        &self,
        class_name: impl Into<String>,
        parser_type: FlowParserTypeEnum,
        content_provider: ContentProvider,
    ) {
        self.registrations
            .write()
            .unwrap()
            .insert(class_name.into(), (parser_type, content_provider));
    }

    /// 返回已注册自定义解析器的格式。
    pub fn parser_type(&self, class_name: &str) -> Option<FlowParserTypeEnum> {
        self.registrations
            .read()
            .unwrap()
            .get(class_name)
            .map(|(parser_type, _)| *parser_type)
    }

    /// 按注册格式创建自定义解析器。
    pub fn create_registered(&self, class_name: &str) -> LFResult<Box<dyn FlowParser>> {
        let parser_type = self.parser_type(class_name).ok_or_else(|| {
            LiteflowError::NodeClassNotFound(format!(
                "custom flow parser is not registered: {class_name}"
            ))
        })?;
        self.create_for_type(class_name, parser_type)
    }

    /// 按指定格式创建自定义解析器，并验证注册格式相容。
    pub fn create_for_type(
        &self,
        class_name: &str,
        parser_type: FlowParserTypeEnum,
    ) -> LFResult<Box<dyn FlowParser>> {
        let registration = self
            .registrations
            .read()
            .unwrap()
            .get(class_name)
            .cloned()
            .ok_or_else(|| {
                LiteflowError::NodeClassNotFound(format!(
                    "custom flow parser is not registered: {class_name}"
                ))
            })?;
        let (registered_type, content_provider) = registration;
        if parser_family(registered_type) != parser_family(parser_type) {
            return Err(LiteflowError::ErrorSupportPath(format!(
                "custom flow parser[{class_name}] registered as {}, requested as {}",
                registered_type.get_type(),
                parser_type.get_type()
            )));
        }

        match parser_family(parser_type) {
            FlowParserTypeEnum::TypeJson => Ok(Box::new(ClassJsonFlowElParser::new(
                self.bus.clone(),
                content_provider,
            ))),
            FlowParserTypeEnum::TypeXml => Ok(Box::new(ClassXmlFlowElParser::new(
                self.bus.clone(),
                content_provider,
            ))),
            FlowParserTypeEnum::TypeYml => Ok(Box::new(ClassYmlFlowElParser::new(
                self.bus.clone(),
                content_provider,
            ))),
            _ => unreachable!("parser_family always returns a canonical non-EL type"),
        }
    }
}

impl FlowParserFactory for ClassParserFactory {
    /// 创建已注册的自定义 JSON EL 解析器。
    fn create_json_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>> {
        self.create_for_type(path, FlowParserTypeEnum::TypeElJson)
    }

    /// 创建已注册的自定义 XML EL 解析器。
    fn create_xml_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>> {
        self.create_for_type(path, FlowParserTypeEnum::TypeElXml)
    }

    /// 创建已注册的自定义 YML EL 解析器。
    fn create_yml_el_parser(&self, path: &str) -> LFResult<Box<dyn FlowParser>> {
        self.create_for_type(path, FlowParserTypeEnum::TypeElYml)
    }
}

fn parser_family(parser_type: FlowParserTypeEnum) -> FlowParserTypeEnum {
    match parser_type {
        FlowParserTypeEnum::TypeJson | FlowParserTypeEnum::TypeElJson => {
            FlowParserTypeEnum::TypeJson
        }
        FlowParserTypeEnum::TypeXml | FlowParserTypeEnum::TypeElXml => FlowParserTypeEnum::TypeXml,
        FlowParserTypeEnum::TypeYml | FlowParserTypeEnum::TypeElYml => FlowParserTypeEnum::TypeYml,
    }
}
