package me.danwi.sqlex.core.jdbc;

import me.danwi.sqlex.core.RepositoryLike;
import me.danwi.sqlex.core.annotation.repository.SqlExConverter;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.type.ParameterConverter;

import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.sql.*;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class ParameterSetter {
    //转换器缓存
    private final Map<Class<?>, ParameterConverter<Object, Object>> parameterConverters;

    private ParameterSetter(Map<Class<?>, ParameterConverter<Object, Object>> parameterConverters) {
        this.parameterConverters = parameterConverters;
    }

    public static ParameterSetter fromRepository(Class<? extends RepositoryLike> repository) {
        Map<Class<?>, ParameterConverter<Object, Object>> parameterConverters = new HashMap<>();
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
                                ParameterConverter<Object, Object> instance;
                                try {
                                    //noinspection unchecked
                                    instance = (ParameterConverter<Object, Object>) converter.getDeclaredConstructor().newInstance();
                                } catch (Exception e) {
                                    throw new SqlExImpossibleException("无法实例化参数类型转换器");
                                }
                                parameterConverters.put((Class<?>) typeArguments[0], instance);
                            }
                        }
                    }
                }
            }
        }
        return new ParameterSetter(parameterConverters);
    }

    private ParameterConverter<Object, Object> getConverterFor(Object parameter) {
        for (Map.Entry<Class<?>, ParameterConverter<Object, Object>> converterEntry : parameterConverters.entrySet()) {
            if (converterEntry.getKey().isInstance(parameter)) {
                return converterEntry.getValue();
            }
        }
        return null;
    }

    //设置单个参数
    public void setParameter(PreparedStatement statement, int index, Object arg) throws SQLException {
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
        } else if (arg instanceof BigInteger) {
            statement.setBigDecimal(index, new BigDecimal((BigInteger) arg));
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
            statement.setTimestamp(index, new java.sql.Timestamp(((java.util.Date) arg).getTime()));
            return;
        } else if (arg instanceof java.time.LocalDate ||
                arg instanceof java.time.LocalTime ||
                arg instanceof java.time.LocalDateTime ||
                arg instanceof java.time.OffsetTime ||
                arg instanceof java.time.OffsetDateTime ||
                arg instanceof java.time.ZonedDateTime
        ) {
            statement.setObject(index, arg);
            return;
        } else if (arg instanceof java.time.Instant) {
            statement.setTimestamp(index, Timestamp.from((java.time.Instant) arg));
            return;
        } else {
            ParameterConverter<Object, Object> converter = getConverterFor(arg);
            if (converter != null) {
                Object convertedArg = converter.convert(arg);
                setParameter(statement, index, convertedArg);
                return;
            }
        }
        throw new SqlExImpossibleException("不支持的参数数据类型");
    }

    /**
     * 将参数设置到预处理语句
     *
     * @param statement 预处理语句
     * @param args      参数
     * @throws SQLException SQL异常
     */
    public void setParameters(PreparedStatement statement, List<Object> args) throws SQLException {
        for (int i = 0; i < args.size(); i++) {
            setParameter(statement, i + 1, args.get(i));
        }
    }
}
