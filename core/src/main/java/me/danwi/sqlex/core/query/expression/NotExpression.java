package me.danwi.sqlex.core.query.expression;

public class NotExpression extends UnaryExpression {
    public NotExpression(Expression exp) {
        super("!", exp);
    }

    @Override
    public String toSQL() {
        if (this.exp instanceof NotVariantExpression)
            return ((NotVariantExpression) this.exp).toNotSQL();
        return super.toSQL();
    }
}
