package me.danwi.sqlex.core.query.expression;

import me.danwi.sqlex.core.exception.SqlExException;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.sql.Timestamp;
import java.time.format.DateTimeFormatter;

public class LiteralExpression implements Expression {
    private final Object value;

    public LiteralExpression(Object value) {
        if (value instanceof Expression)
            throw new SqlExException("已经是一个表达式了,无需转换成预处理参数");
        this.value = value;
    }

    @Override
    public String toSQL() {
        if (
                value instanceof Boolean
                        || value instanceof Byte
                        || value instanceof Short
                        || value instanceof Integer
                        || value instanceof Long
                        || value instanceof Float
                        || value instanceof Double
                        || value instanceof BigInteger
                        || value instanceof BigDecimal
        ) {
            //基本类型
            return value.toString();
        } else if (value instanceof Character
                || value instanceof String) {
            //字符类型
            String literal = value.toString().replace("'", "\\'");
            return "'" + literal + "'";
        } else if (value instanceof java.sql.Date || value instanceof java.time.LocalDate) {
            return String.format("DATE'%s'", value);
        } else if (value instanceof java.sql.Time || value instanceof java.time.LocalTime) {
            return String.format("TIME'%s'", value);
        } else if (value instanceof java.sql.Timestamp) {
            java.sql.Timestamp time = (java.sql.Timestamp) value;
            long second = time.getTime() / 1000;
            long ms = time.getTime() - second * 1000;
            return String.format("FROM_UNIXTIME(%d.%06d)", second, ms * 1000);
        } else if (value instanceof java.util.Date) {
            java.util.Date time = (java.util.Date) value;
            long second = time.getTime() / 1000;
            long ms = time.getTime() - second * 1000;
            return String.format("FROM_UNIXTIME(%d.%06d)", second, ms * 1000);
        } else if (value instanceof java.time.LocalDateTime) {
            java.time.LocalDateTime localDateTime = (java.time.LocalDateTime) value;
            String date = localDateTime.toLocalDate().toString();
            String time = localDateTime.toLocalTime().format(DateTimeFormatter.ofPattern("HH:mm:ss"));
            return String.format("TIMESTAMP('%s %s')", date, time);
        } else if (value instanceof java.time.OffsetDateTime) {
            java.time.OffsetDateTime offsetDateTime = (java.time.OffsetDateTime) value;
            Timestamp timestamp = Timestamp.from(offsetDateTime.toInstant());
            long second = timestamp.getTime() / 1000;
            long ms = timestamp.getTime() - second * 1000;
            return String.format("FROM_UNIXTIME(%d.%06d)", second, ms * 1000);
        } else if (value instanceof java.time.ZonedDateTime) {
            java.time.ZonedDateTime offsetDateTime = (java.time.ZonedDateTime) value;
            Timestamp timestamp = Timestamp.from(offsetDateTime.toInstant());
            long second = timestamp.getTime() / 1000;
            long ms = timestamp.getTime() - second * 1000;
            return String.format("FROM_UNIXTIME(%d.%06d)", second, ms * 1000);
        } else {
            throw new SqlExException("不支持的字面量类型");
        }
    }
}
