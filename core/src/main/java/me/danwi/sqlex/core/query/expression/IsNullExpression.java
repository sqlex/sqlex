package me.danwi.sqlex.core.query.expression;

public class IsNullExpression implements NotVariantExpression {
    private final Expression expression;

    public IsNullExpression(Expression expression) {
        this.expression = expression;
    }

    @Override
    public String toSQL() {
        return String.format("(%s) is null", expression.toSQL());
    }

    @Override
    public String toNotSQL() {
        return String.format("(%s) is not null", expression.toSQL());
    }
}
