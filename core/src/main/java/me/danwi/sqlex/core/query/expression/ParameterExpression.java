package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.exception.SqlExException;

public class ParameterExpression implements Expression {
    private final Object value;

    public ParameterExpression(Object value) {
        if (value instanceof Expression)
            throw new SqlExException("已经是一个表达式了,无需转换成预处理参数");
        this.value = value;
    }

    @Override
    public String toSQL() {
        return ExpressionUtil.getParameterPlaceholder(value);
    }
}
