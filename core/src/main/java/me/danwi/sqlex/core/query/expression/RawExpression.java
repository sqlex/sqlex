package me.danwi.sqlex.core.query.expression;

public class RawExpression implements Expression {
    private final String rawSQL;

    public RawExpression(String rawSQL) {
        this.rawSQL = rawSQL;
    }

    @Override
    public String toSQL() {
        return this.rawSQL;
    }
}
