package me.danwi.sqlex.core.query.expression.logical;

import me.danwi.sqlex.core.query.expression.Expression;

public class NotExpression implements Expression {
    private final Expression exp;

    public NotExpression(Expression exp) {
        this.exp = exp;
    }

    @Override
    public String toSQL() {
        return String.format("!(%s)", exp.toSQL());
    }
}
