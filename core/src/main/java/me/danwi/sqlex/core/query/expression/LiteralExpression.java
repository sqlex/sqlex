package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.exception.SqlExException;

import java.math.BigDecimal;
import java.math.BigInteger;

public class LiteralExpression implements Expression {
    private final Object value;

    public LiteralExpression(Object value) {
        if (value instanceof Expression)
            throw new SqlExException("已经是一个表达式了,无需转换成预处理参数");
        this.value = value;
    }

    @Override
    public String toSQL() {
        if (
                value instanceof Boolean
                        || value instanceof Byte
                        || value instanceof Short
                        || value instanceof Integer
                        || value instanceof Long
                        || value instanceof Float
                        || value instanceof Double
                        || value instanceof BigInteger
                        || value instanceof BigDecimal
        ) {
            //基本类型
            return value.toString();

        } else if (value instanceof Character
                || value instanceof String) {
            //字符类型
            String literal = value.toString().replace("'", "\\'");
            return "'" + literal + "'";
        } else
            throw new SqlExException("不支持的字面量类型");
    }
}
