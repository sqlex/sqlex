package me.danwi.sqlex.core.query.expression;

public class ParameterExpression implements Expression {
    private final Object value;

    public ParameterExpression(Object value) {
        this.value = value;
    }

    @Override
    public String toSQL() {
        return ExpressionUtil.getParameterPlaceholder(value);
    }
}
