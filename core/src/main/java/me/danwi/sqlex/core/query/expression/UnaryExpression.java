package me.danwi.sqlex.core.query.expression;

public class UnaryExpression implements Expression {
    private final String operator;
    protected final Expression exp;

    public UnaryExpression(String operator, Expression exp) {
        this.operator = operator;
        this.exp = exp;
    }

    @Override
    public String toSQL() {
        return String.format("%s(%s)", operator, exp.toSQL());
    }
}
