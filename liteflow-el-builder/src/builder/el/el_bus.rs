use super::el_wrapper::{
    BoxedELWrapper, ELBuilderError, ELBuilderResult, IntoELWrapper, WrapperKind,
};
use super::{
    AndELWrapper, CatchELWrapper, CommonNodeELWrapper, IfELWrapper, LoopELWrapper, NodeELWrapper,
    NotELWrapper, OrELWrapper, ParELWrapper, SerELWrapper, SwitchELWrapper, ThenELWrapper,
    WhenELWrapper,
};

/// EL 表达式链式构建入口。
///
/// Java 依赖运行时重载和 `instanceof`，Rust 版使用泛型转换与显式 `Result`
/// 完成同等校验，不接受无法转换的任意对象。
/// 对应 Java: `com.yomahub.liteflow.builder.el.ELBus`。
pub struct ELBus;

impl ELBus {
    /// 格式化输出使用的缩进字符。
    pub const TAB: &'static str = "\t";

    /// 创建 THEN 串行组件。
    pub fn then<I, T>(items: I) -> ELBuilderResult<ThenELWrapper>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        Ok(ThenELWrapper::new(Self::convert_non_boolean(items)?))
    }

    /// 创建 WHEN 并行组件。
    pub fn when<I, T>(items: I) -> ELBuilderResult<WhenELWrapper>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        Ok(WhenELWrapper::new(Self::convert_non_boolean(items)?))
    }

    /// 创建 SER 串行组件。
    pub fn ser<I, T>(items: I) -> ELBuilderResult<SerELWrapper>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        Ok(SerELWrapper::new(Self::convert_non_boolean(items)?))
    }

    /// 创建 PAR 并行组件。
    pub fn par<I, T>(items: I) -> ELBuilderResult<ParELWrapper>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        Ok(ParELWrapper::new(Self::convert_non_boolean(items)?))
    }

    /// 创建含 ELSE 分支的 IF 条件表达式。
    pub fn if_opt<C, T, F>(
        condition: C,
        true_branch: T,
        false_branch: F,
    ) -> ELBuilderResult<IfELWrapper>
    where
        C: IntoELWrapper,
        T: IntoELWrapper,
        F: IntoELWrapper,
    {
        Ok(IfELWrapper::new(
            Self::convert_one_boolean(condition)?,
            Self::convert_one_non_boolean(true_branch)?,
            Some(Self::convert_one_non_boolean(false_branch)?),
        ))
    }

    /// 创建暂不含 ELSE 分支的 IF 条件表达式。
    pub fn if_then<C, T>(condition: C, true_branch: T) -> ELBuilderResult<IfELWrapper>
    where
        C: IntoELWrapper,
        T: IntoELWrapper,
    {
        Ok(IfELWrapper::new(
            Self::convert_one_boolean(condition)?,
            Self::convert_one_non_boolean(true_branch)?,
            None,
        ))
    }

    /// 创建普通节点表达式。
    pub fn element(node_id: impl Into<String>) -> CommonNodeELWrapper {
        CommonNodeELWrapper::new(node_id)
    }

    /// 创建显式 `node("id")` 单节点表达式。
    pub fn node(node_id: impl Into<String>) -> NodeELWrapper {
        NodeELWrapper::new(node_id)
    }

    /// 创建 SWITCH 选择表达式。
    pub fn switch_opt<T: IntoELWrapper>(selector: T) -> ELBuilderResult<SwitchELWrapper> {
        let selector = selector.into_el_wrapper();
        if selector.wrapper_kind() != WrapperKind::CommonNode {
            return Err(ELBuilderError::InvalidParameter(
                "SWITCH 判断位置只允许普通节点".to_string(),
            ));
        }
        Ok(SwitchELWrapper::new(selector))
    }

    /// 创建以固定次数驱动的 FOR 循环表达式。
    pub fn for_opt_count(loop_number: u32) -> LoopELWrapper {
        LoopELWrapper::for_count(loop_number)
    }

    /// 创建以节点返回值驱动的 FOR 循环表达式。
    pub fn for_opt<T: IntoELWrapper>(source: T) -> ELBuilderResult<LoopELWrapper> {
        let source = source.into_el_wrapper();
        if source.wrapper_kind() != WrapperKind::CommonNode {
            return Err(ELBuilderError::InvalidParameter(
                "FOR 循环次数来源只允许普通节点".to_string(),
            ));
        }
        Ok(LoopELWrapper::for_expression(source))
    }

    /// 创建 WHILE 条件循环表达式。
    pub fn while_opt<T: IntoELWrapper>(condition: T) -> ELBuilderResult<LoopELWrapper> {
        Ok(LoopELWrapper::while_expression(Self::convert_one_boolean(
            condition,
        )?))
    }

    /// 创建 ITERATOR 迭代循环表达式。
    pub fn iterator_opt<T: IntoELWrapper>(source: T) -> ELBuilderResult<LoopELWrapper> {
        let source = source.into_el_wrapper();
        if source.wrapper_kind() != WrapperKind::CommonNode {
            return Err(ELBuilderError::InvalidParameter(
                "ITERATOR 迭代来源只允许普通节点".to_string(),
            ));
        }
        Ok(LoopELWrapper::iterator_expression(source))
    }

    /// 创建 CATCH 异常捕获表达式。
    pub fn catch_exception<T: IntoELWrapper>(body: T) -> ELBuilderResult<CatchELWrapper> {
        Ok(CatchELWrapper::new(Self::convert_one_non_boolean(body)?))
    }

    /// 创建 AND 布尔表达式。
    pub fn and<I, T>(items: I) -> ELBuilderResult<AndELWrapper>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        Ok(AndELWrapper::new(Self::convert_boolean(items)?))
    }

    /// 创建 OR 布尔表达式。
    pub fn or<I, T>(items: I) -> ELBuilderResult<OrELWrapper>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        Ok(OrELWrapper::new(Self::convert_boolean(items)?))
    }

    /// 创建 NOT 布尔表达式。
    pub fn not<T: IntoELWrapper>(item: T) -> ELBuilderResult<NotELWrapper> {
        Ok(NotELWrapper::new(Self::convert_one_boolean(item)?))
    }

    /// 把字符串或包装器集合转换为盒装表达式。
    pub fn convert<I, T>(items: I) -> Vec<BoxedELWrapper>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        items
            .into_iter()
            .map(IntoELWrapper::into_el_wrapper)
            .collect()
    }

    /// 转换并检查所有参数均能返回布尔值。
    pub fn convert_boolean<I, T>(items: I) -> ELBuilderResult<Vec<BoxedELWrapper>>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        let wrappers = Self::convert(items);
        Self::check_boolean_args(&wrappers)?;
        Ok(wrappers)
    }

    /// 转换并检查所有参数均不是 AND/OR/NOT 运算表达式。
    pub fn convert_non_boolean<I, T>(items: I) -> ELBuilderResult<Vec<BoxedELWrapper>>
    where
        I: IntoIterator<Item = T>,
        T: IntoELWrapper,
    {
        let wrappers = Self::convert(items);
        Self::check_not_boolean_args(&wrappers)?;
        Ok(wrappers)
    }

    pub(crate) fn convert_one_boolean<T: IntoELWrapper>(
        item: T,
    ) -> ELBuilderResult<BoxedELWrapper> {
        let wrapper = item.into_el_wrapper();
        Self::check_boolean_args(std::slice::from_ref(&wrapper))?;
        Ok(wrapper)
    }

    pub(crate) fn convert_one_non_boolean<T: IntoELWrapper>(
        item: T,
    ) -> ELBuilderResult<BoxedELWrapper> {
        let wrapper = item.into_el_wrapper();
        Self::check_not_boolean_args(std::slice::from_ref(&wrapper))?;
        Ok(wrapper)
    }

    /// 检查参数都不是 AND/OR/NOT 表达式。
    pub fn check_not_boolean_args(wrappers: &[BoxedELWrapper]) -> ELBuilderResult<()> {
        if wrappers
            .iter()
            .any(|wrapper| wrapper.wrapper_kind().is_boolean_operator())
        {
            return Err(ELBuilderError::InvalidParameter(
                "此位置不允许 AND/OR/NOT 布尔运算表达式".to_string(),
            ));
        }
        Ok(())
    }

    /// 检查参数都能返回布尔值。
    pub fn check_boolean_args(wrappers: &[BoxedELWrapper]) -> ELBuilderResult<()> {
        if wrappers
            .iter()
            .any(|wrapper| !wrapper.wrapper_kind().is_boolean_capable())
        {
            return Err(ELBuilderError::InvalidParameter(
                "此位置只允许普通节点或 AND/OR/NOT 表达式".to_string(),
            ));
        }
        Ok(())
    }
}
