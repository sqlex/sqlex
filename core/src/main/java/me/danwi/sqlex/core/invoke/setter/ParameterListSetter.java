package me.danwi.sqlex.core.invoke.setter;

import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.type.ParameterConverter;

import java.math.BigDecimal;
import java.sql.*;

public class ParameterListSetter implements ParameterSetter {
    final ParameterConverterRegistry registry;

    public ParameterListSetter(ParameterConverterRegistry registry) {
        this.registry = registry;
    }

    @Override
    public void setParameters(PreparedStatement statement, Object[] args) throws SQLException {
        for (int i = 0; i < args.length; i++) {
            setParameter(statement, i, args[i]);
        }
    }

    private void setParameter(PreparedStatement statement, int index, Object arg) throws SQLException {
        if (arg == null) {
            statement.setNull(index, Types.NULL);
            return;
        } else if (arg instanceof Boolean) {
            statement.setBoolean(index, (Boolean) arg);
            return;
        } else if (arg instanceof Byte) {
            statement.setByte(index, (Byte) arg);
            return;
        } else if (arg instanceof Short) {
            statement.setShort(index, (Short) arg);
            return;
        } else if (arg instanceof Integer) {
            statement.setInt(index, (Integer) arg);
            return;
        } else if (arg instanceof Long) {
            statement.setLong(index, (Long) arg);
            return;
        } else if (arg instanceof Float) {
            statement.setFloat(index, (Float) arg);
            return;
        } else if (arg instanceof Double) {
            statement.setDouble(index, (Double) arg);
            return;
        } else if (arg instanceof Character) {
            statement.setString(index, arg.toString());
            return;
        } else if (arg instanceof String) {
            statement.setString(index, (String) arg);
            return;
        } else if (arg instanceof BigDecimal) {
            statement.setBigDecimal(index, (BigDecimal) arg);
            return;
        } else if (arg instanceof byte[]) {
            statement.setBytes(index, (byte[]) arg);
            return;
        } else if (arg instanceof Blob) {
            statement.setBlob(index, (Blob) arg);
            return;
        } else if (arg instanceof java.sql.Date) {
            statement.setDate(index, (java.sql.Date) arg);
            return;
        } else if (arg instanceof java.sql.Time) {
            statement.setTime(index, (java.sql.Time) arg);
            return;
        } else if (arg instanceof java.sql.Timestamp) {
            statement.setTimestamp(index, (java.sql.Timestamp) arg);
            return;
        } else if (arg instanceof java.util.Date) {
            statement.setTimestamp(index, new Timestamp(((java.util.Date) arg).getTime()));
            return;
        } else {
            ParameterConverter<Object, Object> converter = registry.getConverterFor(arg);
            if (converter != null) {
                Object convertedArg = converter.convert(arg);
                setParameter(statement, index, convertedArg);
                return;
            }
        }

        //TODO:优化异常
        throw new SQLException("不支持的参数数据类型");
    }
}
