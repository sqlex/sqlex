package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Method;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.List;

public class SelectOneRowMethodProxy extends SelectMethodProxy {
    public SelectOneRowMethodProxy(Method method, TransactionManager transactionManager, ParameterConverterRegistry registry, ExceptionTranslator translator) {
        super(method, transactionManager, registry, translator);
    }

    @Override
    protected Class<?> getEntityType(Method method) {
        return method.getReturnType();
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws SQLException {
        Object result = super.invoke(args, connection);
        if (result instanceof List) {
            List<?> resultList = (List<?>) result;
            if (resultList.size() > 0)
                return resultList.get(0);
        }
        return null;
    }
}
