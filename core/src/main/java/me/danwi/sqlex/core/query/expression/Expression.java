package me.danwi.sqlex.core.query.expression;

import java.util.Arrays;

public interface Expression {
    /**
     * 表达式转换成SQL片段
     *
     * @return SQL判断
     */
    String toSQL();

    //函数调用
    static FunctionCallExpression func(String name, Expression... args) {
        return new FunctionCallExpression(name, Arrays.asList(args));
    }

    //预处理参数
    static ParameterExpression arg(Object value) {
        return new ParameterExpression(value);
    }

    //逻辑运算
    static UnaryExpression not(Expression exp) {
        return new UnaryExpression("!", exp);
    }

    default BinaryExpression and(Expression right) {
        return new BinaryExpression("and", this, right);
    }

    default BinaryExpression or(Expression right) {
        return new BinaryExpression("or", this, right);
    }

    //关系运算
    default BinaryExpression eq(Expression right) {
        return new BinaryExpression("=", this, right);
    }

    default BinaryExpression gt(Expression right) {
        return new BinaryExpression(">", this, right);
    }

    default BinaryExpression gte(Expression right) {
        return new BinaryExpression(">=", this, right);
    }

    default BinaryExpression lt(Expression right) {
        return new BinaryExpression("<", this, right);
    }

    default BinaryExpression lte(Expression right) {
        return new BinaryExpression("<=", this, right);
    }

    //数学运算
    default BinaryExpression add(Expression right) {
        return new BinaryExpression("+", this, right);
    }

    default BinaryExpression sub(Expression right) {
        return new BinaryExpression("-", this, right);
    }

    default BinaryExpression mul(Expression right) {
        return new BinaryExpression("*", this, right);
    }

    default BinaryExpression div(Expression right) {
        return new BinaryExpression("/", this, right);
    }

    //其他
    default BinaryExpression like(Expression right) {
        return new BinaryExpression("like", this, right);
    }
}
