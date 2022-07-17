package me.danwi.sqlex.core.query.expression;

public class LiteralExpression implements Expression {
    private final Object value;

    public LiteralExpression(Object value) {
        this.value = value;
    }

    @Override
    public String toSQL() {
        if (value instanceof String)
            return "'" + value.toString().replace("'", "\\'") + "'";
        else
            //TODO 其他
            return value.toString();
    }
}
