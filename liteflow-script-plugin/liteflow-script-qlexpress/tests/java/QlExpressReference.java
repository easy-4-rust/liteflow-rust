import com.alibaba.qlexpress4.Express4Runner;
import com.alibaba.qlexpress4.InitOptions;
import com.alibaba.qlexpress4.QLOptions;
import com.alibaba.qlexpress4.QLResult;
import com.alibaba.qlexpress4.security.QLSecurityStrategy;

import java.util.HashMap;
import java.util.Map;

/**
 * 使用 LiteFlow v2.16.0 对应的 QLExpress 4.1.0 计算差分测试基准。
 */
public final class QlExpressReference {

    /** 模拟 LiteFlow 执行时按名称绑定的业务上下文 Bean。 */
    public static final class OrderContext {

        private int orderType;

        public int getOrderType() {
            return orderType;
        }

        public void setOrderType(int orderType) {
            this.orderType = orderType;
        }
    }

    /** 模拟 LiteFlow DefaultContext 的最小真实脚本接口。 */
    public static final class DefaultContext {

        private final Map<String, Object> data = new HashMap<>();

        public Object getData(String key) {
            return data.get(key);
        }

        public boolean hasData(String key) {
            return data.containsKey(key);
        }

        public void setData(String key, Object value) {
            data.put(key, value);
        }
    }

    private static Object execute(
            Express4Runner runner,
            DefaultContext defaultContext,
            Map<String, Object> contextBeans,
            String script) {
        Map<String, Object> bindings = new HashMap<>();
        bindings.put("defaultContext", defaultContext);
        bindings.putAll(contextBeans);
        QLResult result = runner.execute(
                script,
                bindings,
                QLOptions.builder().cache(true).build());
        return result.getResult();
    }

    public static void main(String[] args) {
        Express4Runner runner = new Express4Runner(
                InitOptions.builder()
                        .securityStrategy(QLSecurityStrategy.open())
                        .build());
        DefaultContext defaultContext = new DefaultContext();
        OrderContext orderContext = new OrderContext();
        Map<String, Object> contextBeans = Map.of("order", orderContext);

        execute(
                runner,
                defaultContext,
                contextBeans,
                "a=3; b=2; defaultContext.setData(\"score\", a*b+84);");
        Object decision = execute(
                runner,
                defaultContext,
                contextBeans,
                "score=defaultContext.getData(\"score\");"
                        + "if(score>=60){return true;}else{return false;}");
        Object route = execute(
                runner,
                defaultContext,
                contextBeans,
                "score=defaultContext.getData(\"score\");"
                        + "if(score>100){return \"fail\";}else{return \"pass\";}");
        Object count = execute(runner, defaultContext, contextBeans, "return 3;");
        execute(
                runner,
                defaultContext,
                contextBeans,
                "a=3; b=2; order.setOrderType(a*b);");

        System.out.println("score=" + defaultContext.getData("score"));
        System.out.println("decision=" + decision);
        System.out.println("route=" + route);
        System.out.println("count=" + count);
        System.out.println("orderType=" + orderContext.getOrderType());
    }
}
