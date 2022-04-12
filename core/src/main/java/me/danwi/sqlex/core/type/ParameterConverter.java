package me.danwi.sqlex.core.type;

import org.jetbrains.annotations.Nullable;

/**
 * 参数类型转换器
 *
 * @param <P> 参数类型
 * @param <D> 预支持类型
 */
public interface ParameterConverter<P, D> {
    @Nullable D convert(@Nullable P parameter);
}