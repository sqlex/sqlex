package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;
import org.jetbrains.annotations.NotNull;

import java.beans.IntrospectionException;
import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.util.List;


public class SelectMethodProxy extends BaseMethodProxy {
    private final BeanMapper beanMapper;

    public SelectMethodProxy(@NotNull Method method, @NotNull TransactionManager transactionManager, @NotNull ParameterConverterRegistry registry) throws SqlExImpossibleException {
        super(method, transactionManager, registry);
        //新建bean mapper
        Class<?> beanType = getBeanType(method);
        if (beanType == null)
            throw new SqlExImpossibleException("无法确定返回值类型");
        try {
            beanMapper = new BeanMapper(beanType);
        } catch (IntrospectionException e) {
            throw new SqlExImpossibleException("无法解析返回值类型");
        }
    }

    protected Class<?> getBeanType(Method method) {
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


    @Override
    protected Object invoke(Object[] args, Connection connection) throws Exception {
        String sql = rewriteSQL(args);
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(args);
            setParameters(statement, reorderArgs);
            //获取到返回值
            try (ResultSet rs = statement.executeQuery()) {
                return beanMapper.fetch(rs);
            }
        }
    }
}