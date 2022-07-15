package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.query.SQLParameterBind;

import java.util.*;
import java.util.stream.Collectors;

/**
 * 表达式辅助类
 */
public class ExpressionUtil {
    private static final ThreadLocal<Map<String, Object>> parameterContext = new ThreadLocal<>();

    /**
     * 获取参数占位符
     *
     * @param value 参数值
     * @return 占位符
     */
    public static String getParameterPlaceholder(Object value) {
        Map<String, Object> parameterContext = ExpressionUtil.parameterContext.get();
        if (parameterContext == null) {
            parameterContext = new HashMap<>();
            ExpressionUtil.parameterContext.set(parameterContext);
        }
        while (true) {
            int id = (new Random().nextInt(900000)) + 100000;
            String placeholder = "#Parameter#{" + id + "}";
            //如果已经存在,则重新生成
            if (parameterContext.containsKey(placeholder))
                continue;
            parameterContext.put(placeholder, value);
            return placeholder;
        }
    }

    /**
     * 表达式转换成SQL,并保留参数绑定信息
     *
     * @param expression 表达式树
     * @return 参数bind信息
     */
    public static SQLParameterBind toSQL(Expression expression) {
        try {
            String sql = expression.toSQL();
            //获取参数上下文信息
            List<Object> parameters = new LinkedList<>();
            Map<String, Object> context = ExpressionUtil.parameterContext.get();
            if (context != null) {
                //对占位符进行排序
                String tempSQL = sql;
                List<Map.Entry<String, Object>> placeholders = context.entrySet().stream()
                        .sorted(Comparator.comparing(it -> tempSQL.indexOf(it.getKey())))
                        .collect(Collectors.toList());
                //SQL改写
                for (Map.Entry<String, Object> placeholder : placeholders) {
                    sql = sql.replace(placeholder.getKey(), "?");
                    parameters.add(placeholder.getValue());
                }
            }
            return new SQLParameterBind(sql, parameters);
        } finally {
            //移除上下文
            ExpressionUtil.parameterContext.remove();
        }
    }
}
