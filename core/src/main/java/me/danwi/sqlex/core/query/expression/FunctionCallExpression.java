package me.danwi.sqlex.core.query.expression;

import java.util.List;
import java.util.stream.Collectors;

public class FunctionCallExpression implements Expression {
    private final String name;

    private final List<Expression> args;

    public FunctionCallExpression(String name, List<Expression> args) {
        this.name = name;
        this.args = args;
    }

    @Override
    public String toSQL() {
        return String.format(
                "%s(%s)", name,
                args.stream().map(Expression::toSQL).collect(Collectors.joining(","))
        );
    }
}
