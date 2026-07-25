//! 订单流程示例：对齐 LiteFlow 官方文档的典型用法。
//! 运行：cargo run --example order_demo

use liteflow_rust::{cmp, FlowBus, LiteflowError};
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    let bus = FlowBus::new();

    // 组件注册（对应 Java 的 @LiteflowComponent("xxx")）
    bus.register("checkStock", cmp(|ctx| async move {
        println!("[checkStock] 检查库存");
        ctx.set_data("stock_ok", json!(true));
        Ok(Value::Null)
    }));
    bus.register("queryPrice", cmp(|ctx| async move {
        println!("[queryPrice] 查询价格");
        ctx.set_data("price", json!(99.5));
        Ok(Value::Null)
    }));
    bus.register("queryCoupon", cmp(|_| async move {
        println!("[queryCoupon] 查询优惠券");
        Ok(json!(10))
    }));
    bus.register("isVip", cmp(|_| async move { Ok(json!(true)) }));
    bus.register("vipDiscount", cmp(|_| async move {
        println!("[vipDiscount] VIP 折扣");
        Ok(Value::Null)
    }));
    bus.register("normalPrice", cmp(|_| async move {
        println!("[normalPrice] 普通定价");
        Ok(Value::Null)
    }));
    bus.register("pay", cmp(|ctx| async move {
        let price: f64 = ctx.get_data_as("price").unwrap_or(0.0);
        println!("[pay] 支付 {price}");
        Ok(Value::Null)
    }));
    bus.register("riskCheck", cmp(|_| async move {
        Err(LiteflowError::Custom("风控拦截模拟".into()))
    }));
    bus.register("riskFallback", cmp(|_| async move {
        println!("[riskFallback] 风控降级处理");
        Ok(Value::Null)
    }));

    // EL 编排（对应 Java 规则文件中的 chain）
    bus.add_chain(
        "orderChain",
        "THEN(checkStock, WHEN(queryPrice, queryCoupon), \
         IF(isVip, vipDiscount).ELSE(normalPrice), \
         CATCH(riskCheck).DO(riskFallback), pay)",
    )
    .unwrap();

    let resp = bus.execute_with_data("orderChain", json!({"orderId": "A001"})).await;
    println!("\nsuccess: {}", resp.is_success());
    println!("steps  : {}", resp.step_str());

    // 热刷新：改为普通用户流程
    bus.reload_chain("orderChain", "THEN(checkStock, WHEN(queryPrice, queryCoupon), normalPrice, pay)")
        .unwrap();
    let resp2 = bus.execute("orderChain").await;
    println!("\nafter reload success: {}", resp2.is_success());
    println!("steps  : {}", resp2.step_str());
}
