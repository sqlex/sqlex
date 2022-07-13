package me.danwi.sqlex.core.query.expression.logical;

import me.danwi.sqlex.core.query.expression.Expression;

public class OrExpression implements Expression {
    private final Expression left;
    private final Expression right;

    public OrExpression(Expression left, Expression right) {
        this.left = left;
        this.right = right;
    }

    @Override
    public String toSQL() {
        return String.format("(%s) or (%s)", left.toSQL(), right.toSQL());
    }
}
