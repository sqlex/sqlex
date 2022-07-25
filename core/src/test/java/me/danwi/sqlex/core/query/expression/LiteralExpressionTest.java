package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.exception.SqlExException;
import org.junit.jupiter.api.Test;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.OffsetTime;
import java.time.ZoneOffset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrowsExactly;

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
    public void fromTime() {
        assertEquals("FROM_UNIXTIME(1658479904.123000)", Expression.lit(new java.util.Date(1658479904123L)).toSQL());
        assertEquals("DATE'2022-08-10'", Expression.lit(java.sql.Date.valueOf("2022-08-10")).toSQL());
        assertEquals("TIME'14:32:12'", Expression.lit(java.sql.Time.valueOf("14:32:12")).toSQL());
        assertEquals("FROM_UNIXTIME(1658479904.001000)", Expression.lit(new java.sql.Timestamp(1658479904001L)).toSQL());
        assertEquals("DATE'2022-08-10'", Expression.lit(java.time.LocalDate.of(2022, 8, 10)).toSQL());
        assertEquals("TIME'14:32:12'", Expression.lit(java.time.LocalTime.of(14, 32, 12)).toSQL());
        assertEquals(
                "TIMESTAMP('2022-07-22 16:52:44.123000')",
                Expression.lit(java.time.LocalDateTime.of(
                        2022, 7, 22,
                        16, 52, 44,
                        123 * 1000 * 1000)
                ).toSQL()
        );
        assertEquals("FROM_UNIXTIME(1658479904.012000)", Expression.lit(java.time.OffsetDateTime.of(
                2022, 7, 22,
                8, 51, 44, 12 * 1000 * 1000,
                ZoneOffset.UTC)).toSQL());
        assertEquals("FROM_UNIXTIME(1658479904.012000)", Expression.lit(java.time.ZonedDateTime.of(
                2022, 7, 22,
                8, 51, 44, 12 * 1000 * 1000,
                ZoneOffset.UTC)).toSQL());
    }

    @Test
    public void fromUnsupported() {
        assertThrowsExactly(SqlExException.class, () -> {
            Expression.lit(OffsetTime.now()).toSQL();
        });
        assertThrowsExactly(SqlExException.class, () -> {
            Expression.lit(OffsetTime.now()).toSQL();
        });
    }
}
