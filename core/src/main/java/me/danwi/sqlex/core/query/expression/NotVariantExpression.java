package me.danwi.sqlex.core.query.expression;

public interface NotVariantExpression extends Expression {
    /**
     * 表达式转换成SQL片段,在Not语境中
     *
     * @return SQL片段
     */
    String toNotSQL();
}
