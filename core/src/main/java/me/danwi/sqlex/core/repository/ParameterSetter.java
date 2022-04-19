package me.danwi.sqlex.core.repository;

import me.danwi.sqlex.core.RepositoryLike;
import me.danwi.sqlex.core.annotation.SqlExConverter;
import me.danwi.sqlex.core.type.ParameterConverter;

import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.math.BigDecimal;
import java.sql.*;
import java.util.HashMap;
import java.util.Map;

public class ParameterSetter {
    //转换器缓存
    final private Map<Class<?>, ParameterConverter<Object, Object>> parameterConverters = new HashMap<>();

    public ParameterSetter(Class<? extends RepositoryLike> repository) throws Exception {
        //获取repository上注册参数类型转换器
        SqlExConverter[] converterAnnotations = repository.getAnnotationsByType(SqlExConverter.class);
        //解析到缓存中
        for (SqlExConverter converterAnnotation : converterAnnotations) {
            Class<?> converter = converterAnnotation.converter();
            //获取他到from type
            Type[] converterInterfaces = converter.getGenericInterfaces();
            for (Type converterInterface : converterInterfaces) {
                if (converterInterface instanceof ParameterizedType) {
                    ParameterizedType parameterizedType = (ParameterizedType) converterInterface;
                    if (parameterizedType.getRawType().getTypeName().equals(ParameterConverter.class.getTypeName())) {
                        Type[] typeArguments = parameterizedType.getActualTypeArguments();
                        if (typeArguments.length == 2) {
                            if (typeArguments[0] instanceof Class) {
                                @SuppressWarnings("unchecked")
                                ParameterConverter<Object, Object> instance = (ParameterConverter<Object, Object>) converter.getDeclaredConstructor().newInstance();
                                parameterConverters.put((Class<?>) typeArguments[0], instance);
                            }
                        }
                    }
                }
            }
        }
    }

    public void setParameter(PreparedStatement statement, Object[] args) throws SQLException {
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
            //去mapping中查找
            for (Map.Entry<Class<?>, ParameterConverter<Object, Object>> converterEntry : parameterConverters.entrySet()) {
                if (converterEntry.getKey().isInstance(arg)) {
                    Object convertedArg = converterEntry.getValue().convert(arg);
                    setParameter(statement, index, convertedArg);
                    return;
                }
            }
        }

        //TODO:优化异常
        throw new SQLException("不支持的参数数据类型");
    }
}
