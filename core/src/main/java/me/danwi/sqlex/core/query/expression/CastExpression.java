package me.danwi.sqlex.core.query.expression;

import java.util.Arrays;
import java.util.stream.Collectors;

public class CastExpression implements Expression {
    public enum Type {
        BINARY,
        CHAR,
        DATE,
        DATETIME,
        DECIMAL,
        DOUBLE,
        FLOAT,
        JSON,
        NCHAR,
        REAL,
        SIGNED,
        TIME,
        UNSIGNED,
        YEAR,
        //TODO: 空间类型
    }

    private final Expression expression;
    private final Type type;
    private final long[] size;

    public CastExpression(Expression expression, Type type) {
        this.expression = expression;
        this.type = type;
        this.size = null;
    }

    public CastExpression(Expression expression, Type type, long length) {
        this.expression = expression;
        this.type = type;
        this.size = new long[]{length};
    }

    public CastExpression(Expression expression, Type type, long precision, long scale) {
        this.expression = expression;
        this.type = type;
        this.size = new long[]{precision, scale};
    }

    @Override
    public String toSQL() {
        String typeStr = size == null ?
                this.type.name() :
                String.format(
                        "%s(%s)",
                        this.type.name(),
                        Arrays.stream(size).mapToObj(it -> it + "").collect(Collectors.joining(","))
                );
        return String.format("cast(%s as %s)", this.expression.toSQL(), typeStr);
    }
}
