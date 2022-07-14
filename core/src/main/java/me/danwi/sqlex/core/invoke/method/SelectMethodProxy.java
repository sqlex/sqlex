package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.annotation.method.SqlExOneColumn;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.invoke.mapper.BeanMapper;
import me.danwi.sqlex.core.invoke.mapper.BasicTypeMapper;
import me.danwi.sqlex.core.invoke.mapper.RowMapper;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.List;


public class SelectMethodProxy extends BaseMethodProxy {
    private final RowMapper rowMapper;

    public SelectMethodProxy(Method method, TransactionManager transactionManager, ParameterConverterRegistry registry, ExceptionTranslator translator) {
        super(method, transactionManager, registry, translator);
        //获取返回值中实体/类型
        Class<?> entityType = getEntityType(method);
        if (entityType == null)
            throw new SqlExImpossibleException("无法确定返回值类型");
        //新建row mapper
        //判断是否为单列
        if (method.getAnnotation(SqlExOneColumn.class) != null) {
            //基本支持的类型
            rowMapper = new BasicTypeMapper(entityType);
        } else {
            //构造类型
            rowMapper = new BeanMapper(entityType);
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

    protected RowMapper getRowMapper() {
        return rowMapper;
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws SQLException {
        String sql = rewriteSQL(args);
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(args);
            setParameters(statement, reorderArgs);
            //获取到返回值
            try (ResultSet rs = statement.executeQuery()) {
                return getRowMapper().fetch(rs);
            }
        }
    }
}
