package me.danwi.sqlex.core.type;

/**
 * 参数类型转换器
 *
 * @param <D> 预支持类型
 * @param <P> 参数类型
 */
public interface ParameterConverter<D, P> {
    D convert(P parameter);
}