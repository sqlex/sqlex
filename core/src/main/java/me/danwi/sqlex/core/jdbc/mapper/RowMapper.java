package me.danwi.sqlex.core.jdbc.mapper;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.LocalTime;
import java.time.OffsetDateTime;
import java.util.List;

public abstract class RowMapper<T> {
    protected <C> C fetchColumn(ResultSet resultSet, int columnIndex, Class<C> dataType) throws SQLException {
        //二进制类型单独处理
        if (dataType.isArray()) {
            Class<?> elementType = dataType.getComponentType();
            if (elementType != null) {
                if (elementType.isPrimitive() && elementType.getSimpleName().equals("byte"))
                    return (C) resultSet.getBytes(columnIndex);
            }
            throw new SqlExImpossibleException("结果类中包含不支持的数据类型: " + dataType);
        }
        //其他类型的处理
        Object value;
        if (Boolean.class.equals(dataType)) {
            value = resultSet.getBoolean(columnIndex);
        } else if (Integer.class.equals(dataType)) {
            value = resultSet.getInt(columnIndex);
        } else if (Long.class.equals(dataType)) {
            value = resultSet.getLong(columnIndex);
        } else if (Float.class.equals(dataType)) {
            value = resultSet.getFloat(columnIndex);
        } else if (Double.class.equals(dataType)) {
            value = resultSet.getDouble(columnIndex);
        } else if (BigDecimal.class.equals(dataType)) {
            value = resultSet.getBigDecimal(columnIndex);
        } else if (BigInteger.class.equals(dataType)) {
            BigDecimal decimal = resultSet.getBigDecimal(columnIndex);
            value = (decimal == null ? null : decimal.toBigInteger());
        } else if (String.class.equals(dataType)) {
            value = resultSet.getString(columnIndex);
        } else if (LocalDate.class.equals(dataType)) {
            value = resultSet.getObject(columnIndex, LocalDate.class);
        } else if (LocalTime.class.equals(dataType)) {
            value = resultSet.getObject(columnIndex, LocalTime.class);
        } else if (LocalDateTime.class.equals(dataType)) {
            value = resultSet.getObject(columnIndex, LocalDateTime.class);
        } else if (OffsetDateTime.class.equals(dataType)) {
            value = resultSet.getObject(columnIndex, OffsetDateTime.class);
        } else {
            throw new SqlExImpossibleException("结果类中包含不支持的数据类型: " + dataType);
        }
        if (resultSet.wasNull())
            value = null;
        return (C) value;
    }

    public abstract List<T> fetch(ResultSet resultSet) throws SQLException;
}
