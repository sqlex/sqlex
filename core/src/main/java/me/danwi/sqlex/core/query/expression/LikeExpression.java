package me.danwi.sqlex.core.query.expression;

public class LikeExpression extends BinaryExpression implements NotVariantExpression {
    public LikeExpression(Expression left, Expression right) {
        super("like", left, right);
    }

    @Override
    public String toNotSQL() {
        return String.format("(%s) not like (%s)", left.toSQL(), right.toSQL());
    }
}
