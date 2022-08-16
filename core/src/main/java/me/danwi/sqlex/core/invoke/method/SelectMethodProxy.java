package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.annotation.method.SqlExOneColumn;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.jdbc.RawSQLExecutor;
import me.danwi.sqlex.core.jdbc.mapper.BasicTypeMapper;
import me.danwi.sqlex.core.jdbc.mapper.BeanMapper;
import me.danwi.sqlex.core.jdbc.mapper.RowMapper;

import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;

public class SelectMethodProxy extends BaseMethodProxy {
    private final RowMapper<?> rowMapper;

    public SelectMethodProxy(Method method, RawSQLExecutor executor) {
        super(method, executor);
        //获取返回值中实体/类型
        Class<?> entityType = getEntityType(method);
        if (entityType == null)
            throw new SqlExImpossibleException("无法确定返回值类型");
        //新建row mapper
        //判断是否为单列
        if (method.getAnnotation(SqlExOneColumn.class) != null) {
            //基本支持的类型
            rowMapper = new BasicTypeMapper<>(entityType);
        } else {
            //构造类型
            rowMapper = new BeanMapper<>(entityType);
        }
    }

    //获取方法返回值中实体/类型
    protected Class<?> getEntityType(Method method) {
        //获取返回值类型,List<T>,PageResult<T>
        Type genericReturnType = method.getGenericReturnType();
        if (genericReturnType instanceof ParameterizedType) {
            Type[] actualTypeArguments = ((ParameterizedType) genericReturnType).getActualTypeArguments();
            if (actualTypeArguments != null)
                if (actualTypeArguments.length == 1)
                    if (actualTypeArguments[0] instanceof Class)
                        return (Class<?>) actualTypeArguments[0];
        }
        return null;
    }

    protected RowMapper<?> getRowMapper() {
        return rowMapper;
    }

    @Override
    public Object invoke(Object[] args) {
        return this.executor.query(getRowMapper(), null, rewriteSQL(args), reorderArgs(args));
    }
}
