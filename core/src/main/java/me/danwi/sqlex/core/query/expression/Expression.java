package me.danwi.sqlex.core.query.expression;

import java.util.Arrays;
import java.util.Iterator;
import java.util.List;
import java.util.stream.Collectors;

public interface Expression {
    /**
     * 表达式转换成SQL片段
     *
     * @return SQL判断
     */
    String toSQL();

    //预处理参数
    static ParameterExpression arg(Object value) {
        return new ParameterExpression(value);
    }

    //字面量
    static LiteralExpression lit(Object value) {
        return new LiteralExpression(value);
    }

    //类型转换
    static CastExpression cast(Expression expression, CastExpression.Type type) {
        return new CastExpression(expression, type);
    }

    static CastExpression cast(Expression expression, CastExpression.Type type, long length) {
        return new CastExpression(expression, type, length);
    }

    static CastExpression cast(Expression expression, CastExpression.Type type, long precision, long scale) {
        return new CastExpression(expression, type, precision, scale);
    }

    //函数调用
    static FunctionCallExpression func(String name, Expression... args) {
        return new FunctionCallExpression(name, Arrays.asList(args));
    }

    //原生SQL
    static RawExpression sql(String rawSQL) {
        return new RawExpression(rawSQL);
    }

    //region 逻辑运算
    static NotExpression not(Expression exp) {
        return new NotExpression(exp);
    }

    default BinaryExpression and(Expression right) {
        return new BinaryExpression("and", this, right);
    }

    default BinaryExpression or(Expression right) {
        return new BinaryExpression("or", this, right);
    }
    //endregion

    //region 关系运算
    default BinaryExpression eq(Expression right) {
        return new BinaryExpression("=", this, right);
    }

    default BinaryExpression ne(Expression right) {
        return new BinaryExpression("<>", this, right);
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

    default InExpression in(Iterable<Expression> set) {
        return new InExpression(this, set);
    }

    default NotExpression notIn(Iterable<Expression> set) {
        return not(this.in(set));
    }

    default LikeExpression like(Expression right) {
        return new LikeExpression(this, right);
    }

    default NotExpression notLike(Expression right) {
        return not(this.like(right));
    }

    default IsNullExpression isNull() {
        return new IsNullExpression(this);
    }

    default NotExpression isNotNull() {
        return not(this.isNull());
    }
    //endregion

    //region 算术运算
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
    //endregion

    //region 时间函数
    static FunctionCallExpression now() {
        return func("now");
    }

    static FunctionCallExpression currentTimestamp() {
        return func("current_timestamp");
    }

    static FunctionCallExpression dateFormat(Expression date, Expression format) {
        return func("date_format", date, format);
    }

    static FunctionCallExpression dateFormat(Expression date, String format) {
        return func("date_format", date, lit(format));
    }
    //endregion

    //#region 辅助函数

    /**
     * 将表达式通过 or 联合起来
     *
     * @param expressions 表达式集合
     * @return 联合后的表达式, 如果集合为空, 则返回null
     */
    static Expression joinByAnd(Iterable<Expression> expressions) {
        Iterator<Expression> iterator = expressions.iterator();
        if (!iterator.hasNext()) return null;
        Expression accumulator = iterator.next();
        while (iterator.hasNext()) {
            Expression expression = iterator.next();
            if (expression != null)
                accumulator = accumulator.and(expression);
        }
        return accumulator;
    }

    /**
     * 将表达式通过 or 联合起来
     *
     * @param expressions 表达式集合
     * @return 联合后的表达式, 如果集合为空, 则返回null
     */
    static Expression joinByOr(Iterable<Expression> expressions) {
        Iterator<Expression> iterator = expressions.iterator();
        if (!iterator.hasNext()) return null;
        Expression accumulator = iterator.next();
        while (iterator.hasNext()) {
            Expression expression = iterator.next();
            if (expression != null)
                accumulator = accumulator.or(expression);
        }
        return accumulator;
    }
    //endregion

    //#region 字符串函数
    static FunctionCallExpression concat(Expression... param) {
        return func("concat", param);
    }

    static FunctionCallExpression concatWs(LiteralExpression expression, Expression... param) {
        List<Expression> list = Arrays.stream(param).collect(Collectors.toList());
        list.add(0, expression);
        return new FunctionCallExpression("concat_ws", list);
    }
    //endregion
}
