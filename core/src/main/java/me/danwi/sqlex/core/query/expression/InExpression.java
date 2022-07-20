package me.danwi.sqlex.core.query.expression;

import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.StreamSupport;

public class InExpression implements NotVariantExpression {
    private final Expression expression;
    private final List<Expression> set;

    public InExpression(Expression expression, Iterable<Expression> set) {
        this.expression = expression;
        this.set = StreamSupport.stream(set.spliterator(), false).collect(Collectors.toList());
    }

    @Override
    public String toSQL() {
        if (set.size() == 0)
            return "1 = 2";
        return String.format("%s in (%s)", expression.toSQL(), set.stream().map(Expression::toSQL).collect(Collectors.joining(", ")));
    }

    @Override
    public String toNotSQL() {
        if (set.size() == 0)
            return "1 = 1";
        return String.format("%s not in (%s)", expression.toSQL(), set.stream().map(Expression::toSQL).collect(Collectors.joining(", ")));
    }
}
