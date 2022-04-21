package me.danwi.sqlex.core.repository;

import me.danwi.sqlex.core.RepositoryLike;
import me.danwi.sqlex.core.annotation.SqlExConverter;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.type.ParameterConverter;

import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.sql.SQLException;
import java.util.HashMap;
import java.util.Map;

public class ParameterConverterRegistry {
    //转换器缓存
    private final Map<Class<?>, ParameterConverter<Object, Object>> parameterConverters;

    private ParameterConverterRegistry(Map<Class<?>, ParameterConverter<Object, Object>> parameterConverters) {
        this.parameterConverters = parameterConverters;
    }

    public static ParameterConverterRegistry fromRepository(Class<? extends RepositoryLike> repository) throws SQLException {
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
                                @SuppressWarnings("unchecked")
                                ParameterConverter<Object, Object> instance = null;
                                try {
                                    //noinspection unchecked
                                    instance = (ParameterConverter<Object, Object>) converter.getDeclaredConstructor().newInstance();
                                } catch (Exception e) {
                                    throw new SqlExImpossibleException("无法实例话参数类型转换器");
                                }
                                parameterConverters.put((Class<?>) typeArguments[0], instance);
                            }
                        }
                    }
                }
            }
        }
        return new ParameterConverterRegistry(parameterConverters);
    }

    public ParameterConverter<Object, Object> getConverterFor(Object parameter) {
        for (Map.Entry<Class<?>, ParameterConverter<Object, Object>> converterEntry : parameterConverters.entrySet()) {
            if (converterEntry.getKey().isInstance(parameter)) {
                return converterEntry.getValue();
            }
        }
        return null;
    }
}
