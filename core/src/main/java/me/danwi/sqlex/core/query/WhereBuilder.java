package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.query.expression.Expression;

import java.util.function.Supplier;

public class WhereBuilder<T> {
    //where条件
    protected Expression whereCondition;

    /**
     * 添加where条件
     *
     * @param exp 条件表达式
     * @return this
     */
    public T where(Expression exp) {
        if (exp == null)
            return (T) this;
        if (whereCondition == null)
            whereCondition = exp;
        else
            whereCondition = whereCondition.and(exp);
        return (T) this;
    }

    /**
     * 添加where条件,条件通过运算获得
     *
     * @param supplier 条件提供者
     * @return this
     * @deprecated 请勿使用该函数, 在以后的版本中, 会删除
     */
    @Deprecated
    public T where(Supplier<Expression> supplier) {
        return where(supplier.get());
    }
}
