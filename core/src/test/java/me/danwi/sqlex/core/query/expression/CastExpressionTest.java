package me.danwi.sqlex.core.query.expression;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

public class CastExpressionTest {
    @Test
    void noLength() {
        assertEquals("cast('1' as BINARY)", Expression.cast(Expression.lit("1"), CastExpression.Type.BINARY).toSQL());
        assertEquals("cast('2020-01-01' as DATE)", Expression.cast(Expression.lit("2020-01-01"), CastExpression.Type.DATE).toSQL());
    }

    @Test
    void oneLength() {
        assertEquals("cast('1' as CHAR(1))", Expression.cast(Expression.lit("1"), CastExpression.Type.CHAR, 1).toSQL());
        assertEquals("cast('1' as FLOAT(1))", Expression.cast(Expression.lit("1"), CastExpression.Type.FLOAT, 1).toSQL());
    }

    @Test
    void twoLength() {
        assertEquals("cast('1' as DECIMAL(10,2))", Expression.cast(Expression.lit("1"), CastExpression.Type.DECIMAL, 10, 2).toSQL());
    }
}
