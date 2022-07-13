package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.query.expression.logical.AndExpression;
import me.danwi.sqlex.core.query.expression.logical.NotExpression;
import me.danwi.sqlex.core.query.expression.logical.OrExpression;

import java.util.Arrays;

public interface Expression {
    /**
     * 表达式转换成SQL片段
     *
     * @return SQL判断
     */
    String toSQL();

    static FunctionCallExpression func(String name, Expression... args) {
        return new FunctionCallExpression(name, Arrays.asList(args));
    }

    static ParameterExpression arg(Object value) {
        return new ParameterExpression(value);
    }

    static NotExpression not(Expression exp) {
        return new NotExpression(exp);
    }

    default AndExpression and(Expression right) {
        return new AndExpression(this, right);
    }

    default OrExpression or(Expression right) {
        return new OrExpression(this, right);
    }
}
