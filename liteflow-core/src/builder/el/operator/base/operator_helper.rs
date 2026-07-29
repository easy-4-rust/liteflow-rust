use crate::el::{Arg, El, Mods, NodeRef};
use crate::enums::NodeTypeEnum;
use crate::exception::{LFResult, LiteflowError};
use crate::flow::element::node::Node;

/// EL 操作符参数校验与转换助手。
///
/// Java 版负责 `Object[]` 数量检查、Class 转换和布尔/普通表达式校验；
/// Rust 版利用 `Arg` 枚举消除 Class 强转，并集中生成一致的错误信息。
/// 对应 Java: `com.yomahub.liteflow.builder.el.operator.base.OperatorHelper`。
pub struct OperatorHelper;

impl OperatorHelper {
    /// 检查参数数量大于零。
    ///
    /// # 参数
    /// - `objects`: 操作符收到的参数。
    ///
    /// # 返回
    /// 参数非空时返回 `Ok(())`，否则返回 Java `ELParseException` 对应的解析错误。
    /// 对应 Java: `OperatorHelper#checkObjectSizeGtZero`。
    pub fn check_object_size_gt_zero<T>(objects: &[T]) -> LFResult<()> {
        if objects.is_empty() {
            return Err(LiteflowError::Parse("parameter is empty".to_string()));
        }
        Ok(())
    }

    /// 检查参数数量大于等于两个。
    ///
    /// # 参数
    /// - `objects`: 操作符收到的参数。
    ///
    /// # 返回
    /// 参数不少于两个时返回 `Ok(())`，否则返回参数数量错误。
    /// 对应 Java: `OperatorHelper#checkObjectSizeGteTwo`。
    pub fn check_object_size_gte_two<T>(objects: &[T]) -> LFResult<()> {
        Self::check_object_size_gt_zero(objects)?;
        if objects.len() < 2 {
            return Err(LiteflowError::Parse("parameter size error".to_string()));
        }
        Ok(())
    }

    /// 检查参数数量等于一个。
    ///
    /// 对应 Java: `OperatorHelper#checkObjectSizeEqOne`。
    pub fn check_object_size_eq_one<T>(objects: &[T]) -> LFResult<()> {
        Self::check_object_size_eq(objects, &[1])
    }

    /// 检查参数数量等于两个。
    ///
    /// 对应 Java: `OperatorHelper#checkObjectSizeEqTwo`。
    pub fn check_object_size_eq_two<T>(objects: &[T]) -> LFResult<()> {
        Self::check_object_size_eq(objects, &[2])
    }

    /// 检查参数数量等于三个。
    ///
    /// 对应 Java: `OperatorHelper#checkObjectSizeEqThree`。
    pub fn check_object_size_eq_three<T>(objects: &[T]) -> LFResult<()> {
        Self::check_object_size_eq(objects, &[3])
    }

    /// 检查参数数量是否命中任一允许值。
    ///
    /// Rust 以 `sizes` 切片统一承载 Java 的单个 `size` 与 `size1/size2` 两个重载。
    ///
    /// # 参数
    /// - `objects`: 操作符收到的参数。
    /// - `sizes`: 允许的参数数量集合。
    ///
    /// 对应 Java: `OperatorHelper#checkObjectSizeEq`。
    pub fn check_object_size_eq<T>(objects: &[T], sizes: &[usize]) -> LFResult<()> {
        Self::check_object_size_gt_zero(objects)?;
        if !sizes.contains(&objects.len()) {
            return Err(LiteflowError::Parse("parameter size error".to_string()));
        }
        Ok(())
    }

    /// 将动态操作符参数转换为独立副本。
    ///
    /// `Arg::Expr` 中的 `NodeRef` 会随 AST 深克隆，避免当前 Chain 的 tag、data、
    /// bind 等局部属性污染 FlowBus 中的注册节点，保留 Java `Node#clone` 的语义。
    ///
    /// # 参数
    /// - `object`: 待转换参数。
    ///
    /// # 返回
    /// 参数的独立副本。Java 的运行期 Class 校验由 Rust 泛型在编译期完成。
    /// 对应 Java: `OperatorHelper#convert`。
    #[must_use]
    pub fn convert<T: Clone>(object: &T) -> T {
        object.clone()
    }

    /// 将数字参数转换为双精度浮点数。
    ///
    /// Rust 词法层已经把所有数字统一解析为 `f64`，因此不会再经历 Java
    /// `Float -> Double` 的二次二进制扩展，也就不会引入额外精度损失。
    ///
    /// # 参数
    /// - `object`: 词法层已经确认的数字参数。
    ///
    /// # 返回
    /// 等值的双精度数字。
    /// 对应 Java: `OperatorHelper#convert2Double`。
    #[must_use]
    pub fn convert2_double(object: f64) -> f64 {
        object
    }

    /// 检查参数数组中不存在空值。
    ///
    /// # 参数
    /// - `objects`: 操作符收到的可空参数；Rust 原生非 `Option` 参数天然非空。
    ///
    /// # 返回
    /// 全部非空时返回 `Ok(())`；发现 `null` 时返回 Java 固定消息
    /// `DataNotFoundException`。
    /// 对应 Java: `OperatorHelper#checkItemNotNull`。
    pub fn check_item_not_null<T>(objects: &[Option<T>]) -> LFResult<()> {
        if objects.iter().any(Option::is_none) {
            return Err(LiteflowError::Parse("DataNotFoundException".to_string()));
        }
        Ok(())
    }

    /// 检查对象可放入普通执行位置。
    ///
    /// Rust AST 尚未绑定 FlowBus 时无法获知节点组件类型，因此这里验证表达式形态；
    /// 真实节点绑定后由 `check_resolved_node` 再执行 Java 的 NodeType 校验。
    ///
    /// # 参数
    /// - `object`: 待检查表达式。
    ///
    /// 对应 Java: `OperatorHelper#checkObjMustBeCommonTypeItem`。
    pub fn check_obj_must_be_common_type_item(object: &El) -> LFResult<()> {
        if matches!(object, El::Boolean(_)) {
            return Err(LiteflowError::Parse(
                "The parameter must be Executable item.".to_string(),
            ));
        }
        Ok(())
    }

    /// 检查对象是否能产生布尔结果。
    ///
    /// Rust 的布尔 AST 包括布尔节点引用、AND/OR/NOT 条件和 WHILE 使用的布尔
    /// 字面量；节点引用的真实类型在绑定 FlowBus 后进行第二阶段校验。
    ///
    /// # 参数
    /// - `object`: 待检查表达式。
    ///
    /// 对应 Java: `OperatorHelper#checkObjMustBeBooleanTypeItem`。
    pub fn check_obj_must_be_boolean_type_item(object: &El) -> LFResult<()> {
        let object = unmodified(object);
        if matches!(
            object,
            El::Node(_) | El::Boolean(_) | El::And(_) | El::Or(_) | El::Not(_)
        ) {
            return Ok(());
        }
        Err(LiteflowError::Parse("The parameter error.".to_string()))
    }

    /// 检查对象是否为 FOR 类型节点引用。
    ///
    /// 对应 Java: `OperatorHelper#checkObjMustBeForTypeItem`。
    pub fn check_obj_must_be_for_type_item(object: &El) -> LFResult<()> {
        check_node_expression(object, "For")
    }

    /// 检查对象是否为 ITERATOR 类型节点引用。
    ///
    /// 对应 Java: `OperatorHelper#checkObjMustBeIteratorTypeItem`。
    pub fn check_obj_must_be_iterator_type_item(object: &El) -> LFResult<()> {
        check_node_expression(object, "Iterator")
    }

    /// 检查对象是否为 SWITCH 类型节点引用。
    ///
    /// 对应 Java: `OperatorHelper#checkObjMustBeSwitchTypeItem`。
    pub fn check_obj_must_be_switch_type_item(object: &El) -> LFResult<()> {
        check_node_expression(object, "Switch")
    }

    /// 对已经从 FlowBus 解析出的真实 Node 执行第二阶段类型校验。
    ///
    /// 未声明 `node_type` 的 Rust 闭包组件沿用现有类型推断兼容行为；显式声明类型
    /// 的组件必须符合当前位置要求。该方法把 Java OperatorHelper 的动态对象检查
    /// 落到 Rust 的真实组件实例上，而不是只检查 AST 外形。
    pub(crate) fn check_resolved_node(
        node: &Node,
        expected_node_type: NodeTypeEnum,
    ) -> LFResult<()> {
        if !node.get_instance().has_explicit_node_type() {
            return Ok(());
        }
        let Some(actual_node_type) = node.get_type() else {
            return Ok(());
        };
        let valid = match expected_node_type {
            NodeTypeEnum::Common => matches!(
                actual_node_type,
                NodeTypeEnum::Common | NodeTypeEnum::Script | NodeTypeEnum::Fallback
            ),
            NodeTypeEnum::Boolean
            | NodeTypeEnum::If
            | NodeTypeEnum::While
            | NodeTypeEnum::Break => matches!(
                actual_node_type,
                NodeTypeEnum::Boolean
                    | NodeTypeEnum::BooleanScript
                    | NodeTypeEnum::If
                    | NodeTypeEnum::IfScript
                    | NodeTypeEnum::While
                    | NodeTypeEnum::WhileScript
                    | NodeTypeEnum::Break
                    | NodeTypeEnum::BreakScript
                    | NodeTypeEnum::Fallback
            ),
            NodeTypeEnum::For => matches!(
                actual_node_type,
                NodeTypeEnum::For | NodeTypeEnum::ForScript | NodeTypeEnum::Fallback
            ),
            NodeTypeEnum::Iterator => {
                matches!(
                    actual_node_type,
                    NodeTypeEnum::Iterator | NodeTypeEnum::Fallback
                )
            }
            NodeTypeEnum::Switch => matches!(
                actual_node_type,
                NodeTypeEnum::Switch | NodeTypeEnum::SwitchScript | NodeTypeEnum::Fallback
            ),
            NodeTypeEnum::Script
            | NodeTypeEnum::SwitchScript
            | NodeTypeEnum::BooleanScript
            | NodeTypeEnum::IfScript
            | NodeTypeEnum::ForScript
            | NodeTypeEnum::WhileScript
            | NodeTypeEnum::BreakScript
            | NodeTypeEnum::Fallback => actual_node_type == expected_node_type,
        };
        if valid {
            return Ok(());
        }

        let node_id = node.get_id();
        let message = match expected_node_type {
            NodeTypeEnum::Common => {
                format!("The node[{node_id}] must be a common type component")
            }
            NodeTypeEnum::Boolean
            | NodeTypeEnum::If
            | NodeTypeEnum::While
            | NodeTypeEnum::Break => {
                format!("The node[{node_id}] must be boolean type Node.")
            }
            NodeTypeEnum::For => format!("The node[{node_id}] must be For type Node."),
            NodeTypeEnum::Iterator => {
                format!("The node[{node_id}] must be Iterator type Node.")
            }
            NodeTypeEnum::Switch => format!("The node[{node_id}] must be Switch type Node."),
            other => format!(
                "The node[{node_id}] must be {} type Node.",
                other.get_code()
            ),
        };
        Err(LiteflowError::Parse(message))
    }

    /// 校验调用者必须为空，即当前操作符只能作为主表达式使用。
    pub(crate) fn require_primary(caller: Option<El>, operator: &str) -> LFResult<()> {
        if caller.is_some() {
            return Err(LiteflowError::Parse(format!(
                "{operator} must be used as a primary expression"
            )));
        }
        Ok(())
    }

    /// 读取后缀操作符的左侧调用表达式。
    pub(crate) fn require_caller(caller: Option<El>, operator: &str) -> LFResult<El> {
        caller.ok_or_else(|| {
            LiteflowError::Parse(format!("{operator} must follow an executable expression"))
        })
    }

    /// 把全部参数转换为表达式，并校验最小数量。
    ///
    /// 字符串参数按节点引用转换，与 Java QLExpress 的 Executable 参数一致。
    pub(crate) fn expressions(
        objects: Vec<Arg>,
        operator: &str,
        minimum: usize,
    ) -> LFResult<Vec<El>> {
        Self::check_args_not_null(&objects)?;
        if minimum == 1 {
            Self::check_object_size_gt_zero(&objects)?;
        } else if minimum == 2 {
            Self::check_object_size_gte_two(&objects)?;
        } else if objects.len() < minimum {
            return Err(LiteflowError::Parse("parameter size error".to_string()));
        }
        let mut expressions = Vec::with_capacity(objects.len());
        for object in objects {
            match Self::convert(&object) {
                Arg::Expr(expression) => expressions.push(expression),
                Arg::Str(node_id) => expressions.push(El::Node(NodeRef::new(node_id))),
                other => {
                    return Err(LiteflowError::Parse(format!(
                        "{operator} requires expression arguments, got {other:?}"
                    )));
                }
            }
        }
        Ok(expressions)
    }

    /// 读取唯一表达式参数。
    pub(crate) fn one_expression(objects: Vec<Arg>, operator: &str) -> LFResult<El> {
        Self::check_object_size_eq_one(&objects)?;
        let mut expressions = Self::expressions(objects, operator, 1)?;
        Ok(expressions.remove(0))
    }

    /// 读取唯一字符串参数。
    pub(crate) fn one_string(objects: Vec<Arg>, operator: &str) -> LFResult<String> {
        Self::check_args_not_null(&objects)?;
        Self::check_object_size_eq_one(&objects)?;
        match objects.as_slice() {
            [Arg::Str(value)] => Ok(value.clone()),
            _ => Err(LiteflowError::Parse(format!(
                "{operator} requires exactly one string"
            ))),
        }
    }

    /// 读取唯一布尔参数。
    pub(crate) fn one_bool(objects: Vec<Arg>, operator: &str) -> LFResult<bool> {
        Self::check_args_not_null(&objects)?;
        Self::check_object_size_eq_one(&objects)?;
        match objects.as_slice() {
            [Arg::Bool(value)] => Ok(*value),
            _ => Err(LiteflowError::Parse(format!(
                "{operator} requires exactly one bool"
            ))),
        }
    }

    /// 读取唯一数字参数。
    pub(crate) fn one_number(objects: Vec<Arg>, operator: &str) -> LFResult<f64> {
        Self::check_args_not_null(&objects)?;
        Self::check_object_size_eq_one(&objects)?;
        match objects[0] {
            Arg::Num(value) => Ok(Self::convert2_double(value)),
            _ => Err(LiteflowError::Parse(format!(
                "{operator} requires exactly one number"
            ))),
        }
    }

    /// 合并通用修饰，避免多次后缀调用形成无意义的嵌套 Mods。
    pub(crate) fn add_mods(expression: El, mods: Mods) -> El {
        // Java QLExpress 按源码顺序执行扩展函数。retry/maxWait 每次都创建一个
        // 新 Condition，必须把此前表达式完整包在内部；若在这里合并字段，
        // `a.maxWait(...).retry(...)` 会被错误重排成 retry 在内、timeout 在外。
        if mods.creates_wrapper_condition() {
            return El::Mods(Box::new(expression), mods);
        }
        match expression {
            El::Mods(inner, mut old) => {
                if mods.id.is_some() {
                    old.id = mods.id;
                }
                if mods.tag.is_some() {
                    old.tag = mods.tag;
                }
                if mods.thread_pool.is_some() {
                    old.thread_pool = mods.thread_pool;
                }
                if mods.retry.is_some() {
                    old.retry = mods.retry;
                }
                if !mods.retry_for.is_empty() {
                    old.retry_for = mods.retry_for;
                }
                if mods.max_wait_ms.is_some() {
                    old.max_wait_ms = mods.max_wait_ms;
                }
                if !mods.bind.is_empty() {
                    for (key, value) in mods.bind {
                        let override_key = mods.bind_override_keys.contains(&key);
                        old.bind.retain(|(existing, _)| *existing != key);
                        old.bind_override_keys.retain(|existing| existing != &key);
                        if override_key {
                            old.bind_override_keys.push(key.clone());
                        }
                        old.bind.push((key, value));
                    }
                }
                El::Mods(inner, old)
            }
            other => El::Mods(Box::new(other), mods),
        }
    }

    /// 让类型化操作符穿透只包含属性的 Mods，并在操作完成后恢复属性。
    ///
    /// Java 的 `id/tag/bind/threadPool` 直接修改同一个 Condition 实例，后续
    /// `ANY/DO/ELSE/TO` 等操作符看到的仍是原具体类型。Rust 用 Mods 保存这些
    /// 位置属性，不能因此把 When/Loop/If/Switch 的类型遮蔽。含 retry/maxWait
    /// 的 Mods 表示真实包装 Condition，必须保持不可穿透。
    pub(crate) fn map_through_property_mods(
        expression: El,
        apply: impl FnOnce(El) -> LFResult<El>,
    ) -> LFResult<El> {
        match expression {
            El::Mods(inner, mods) if !mods.creates_wrapper_condition() => {
                apply(*inner).map(|mapped| Self::add_mods(mapped, mods))
            }
            other => apply(other),
        }
    }

    /// 把内部动态参数映射为公开 `Option<T>` 空值校验入口。
    pub(crate) fn check_args_not_null(objects: &[Arg]) -> LFResult<()> {
        let nullable_objects = objects
            .iter()
            .map(|object| (!matches!(object, Arg::Null)).then_some(object))
            .collect::<Vec<_>>();
        Self::check_item_not_null(&nullable_objects)
    }
}

/// 去除通用修饰包装，取得需要执行类型检查的真实表达式。
fn unmodified(mut object: &El) -> &El {
    while let El::Mods(inner, _) = object {
        object = inner;
    }
    object
}

/// 检查表达式最终是否为节点引用。
fn check_node_expression(object: &El, expected: &str) -> LFResult<()> {
    if matches!(unmodified(object), El::Node(_)) {
        return Ok(());
    }
    Err(LiteflowError::Parse(format!(
        "The parameter error. It must be {expected} type Node."
    )))
}
