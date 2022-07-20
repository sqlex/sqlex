package me.danwi.sqlex.core.query.expression;

public class BinaryExpression implements Expression {
    private final String operator;
    protected final Expression left;
    protected final Expression right;

    public BinaryExpression(String operator, Expression left, Expression right) {
        this.operator = operator;
        this.left = left;
        this.right = right;
    }

    @Override
    public String toSQL() {
        return String.format("(%s) %s (%s)", left.toSQL(), operator, right.toSQL());
    }
}
