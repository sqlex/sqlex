package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.beans.IntrospectionException;
import java.lang.reflect.Method;
import java.sql.Connection;
import java.util.List;

public class SelectOneRowMethodProxy extends SelectMethodProxy {
    public SelectOneRowMethodProxy(Method method, TransactionManager transactionManager, ParameterConverterRegistry registry) throws SqlExImpossibleException, IntrospectionException {
        super(method, transactionManager, registry);
    }

    @Override
    protected Class<?> getBeanType(Method method) {
        return method.getReturnType();
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws Exception {
        Object result = super.invoke(args, connection);
        if (result instanceof List) {
            List<?> resultList = (List<?>) result;
            if (resultList.size() > 0)
                return resultList.get(0);
        }
        return null;
    }
}
