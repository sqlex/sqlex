package me.danwi.sqlex.core.jdbc.mapper;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;

import java.math.BigDecimal;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.List;

public abstract class RowMapper<T> {
    protected T fetchColumn(ResultSet resultSet, int columnIndex, Class<?> dataType) throws SQLException {
        return fetchColumn(resultSet, columnIndex, dataType.getName());
    }

    protected T fetchColumn(ResultSet resultSet, int colIndex, String dataTypeName) throws SQLException {
        Object value;
        switch (dataTypeName) {
            case "java.lang.Boolean":
                value = resultSet.getBoolean(colIndex);
                break;
            case "java.lang.Integer":
                value = resultSet.getInt(colIndex);
                break;
            case "java.lang.Long":
                value = resultSet.getLong(colIndex);
                break;
            case "java.lang.Float":
                value = resultSet.getFloat(colIndex);
                break;
            case "java.lang.Double":
                value = resultSet.getDouble(colIndex);
                break;
            case "java.math.BigDecimal":
                value = resultSet.getBigDecimal(colIndex);
                break;
            case "java.math.BigInteger":
                BigDecimal decimal = resultSet.getBigDecimal(colIndex);
                value = (decimal == null ? null : decimal.toBigInteger());
                break;
            case "java.lang.String":
                value = resultSet.getString(colIndex);
                break;
            case "java.time.LocalDate":
                value = resultSet.getObject(colIndex, java.time.LocalDate.class);
                break;
            case "java.time.LocalTime":
                value = resultSet.getObject(colIndex, java.time.LocalTime.class);
                break;
            case "java.time.LocalDateTime":
                value = resultSet.getObject(colIndex, java.time.LocalDateTime.class);
                break;
            case "java.time.OffsetDateTime":
                value = resultSet.getObject(colIndex, java.time.OffsetDateTime.class);
                break;
            default:
                throw new SqlExImpossibleException("结果类中包含不支持的数据类型: " + dataTypeName);
        }
        if (resultSet.wasNull())
            value = null;
        return (T) value;
    }


    public abstract List<T> fetch(ResultSet resultSet) throws SQLException;
}
