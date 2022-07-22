package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.exception.SqlExException;
import org.junit.jupiter.api.Test;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.OffsetDateTime;

import static org.junit.jupiter.api.Assertions.*;

public class LiteralExpressionTest {
    @Test
    public void fromBoolean() {
        assertEquals("true", Expression.lit(true).toSQL());
        assertEquals("false", Expression.lit(false).toSQL());
    }

    @Test
    public void fromNumber() {
        assertEquals("1", Expression.lit((byte) 1).toSQL());
        assertEquals("-1", Expression.lit((byte) -1).toSQL());
        assertEquals("127", Expression.lit((byte) 127).toSQL());
        assertEquals("-128", Expression.lit((byte) -128).toSQL());
        assertEquals("-128", Expression.lit((byte) 128).toSQL());

        assertEquals("1", Expression.lit((short) 1).toSQL());
        assertEquals("-1", Expression.lit((short) -1).toSQL());
        assertEquals("127", Expression.lit((short) 127).toSQL());
        assertEquals("-128", Expression.lit((short) -128).toSQL());
        assertEquals("128", Expression.lit((short) 128).toSQL());
        assertEquals("10086", Expression.lit((short) 10086).toSQL());

        assertEquals("1", Expression.lit(1).toSQL());
        assertEquals("123123123", Expression.lit(123123123).toSQL());
        assertEquals("-123123123", Expression.lit(-123123123).toSQL());

        assertEquals("1", Expression.lit(1L).toSQL());
        assertEquals("-1", Expression.lit(-1L).toSQL());
        assertEquals("127", Expression.lit(127L).toSQL());
        assertEquals("-128", Expression.lit(-128L).toSQL());
        assertEquals("128", Expression.lit(128L).toSQL());
        assertEquals("10086", Expression.lit(10086L).toSQL());
        assertEquals("123123123123", Expression.lit(123123123123L).toSQL());

        assertEquals("1.0", Expression.lit(1.0F).toSQL());
        assertEquals("-1.0", Expression.lit(-1.0F).toSQL());
        assertEquals("3.141592", Expression.lit(3.141592F).toSQL());
        assertEquals("-3.141592", Expression.lit(-3.141592F).toSQL());

        assertEquals("1.0", Expression.lit(1.0).toSQL());
        assertEquals("-1.0", Expression.lit(-1.0).toSQL());
        assertEquals("3.1415926", Expression.lit(3.1415926).toSQL());
        assertEquals("-3.1415926", Expression.lit(-3.1415926).toSQL());
        assertEquals("3.141592653", Expression.lit(3.141592653).toSQL());
        assertEquals("-3.141592653", Expression.lit(-3.141592653).toSQL());

        assertEquals("31415926", Expression.lit(new BigInteger("31415926")).toSQL());
        assertEquals("-31415926", Expression.lit(new BigInteger("-31415926")).toSQL());

        assertEquals("3.1415926", Expression.lit(new BigDecimal("3.1415926")).toSQL());
        assertEquals("-3.1415926", Expression.lit(new BigDecimal("-3.1415926")).toSQL());
        assertEquals("-3.141592600", Expression.lit(new BigDecimal("-3.141592600")).toSQL());
        assertEquals("-3.14159260000", Expression.lit(new BigDecimal("-3.14159260000")).toSQL());
    }

    @Test
    public void fromString() {
        assertEquals("'a'", Expression.lit('a').toSQL());
        assertEquals("'\\''", Expression.lit('\'').toSQL());

        assertEquals("'string'", Expression.lit("string").toSQL());
        assertEquals("'\\'string'", Expression.lit("'string").toSQL());
    }

    @Test
    public void fromUnsupported() {
        assertThrowsExactly(SqlExException.class, () -> {
            Expression.lit(OffsetDateTime.now()).toSQL();
        });
        assertThrowsExactly(SqlExException.class, () -> {
            Expression.lit(OffsetDateTime.now()).toSQL();
        });
    }
}
