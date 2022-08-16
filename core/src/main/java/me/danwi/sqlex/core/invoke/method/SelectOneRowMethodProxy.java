package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

import java.lang.reflect.Method;
import java.util.List;

public class SelectOneRowMethodProxy extends SelectMethodProxy {
    public SelectOneRowMethodProxy(Method method, RawSQLExecutor executor) {
        super(method, executor);
    }

    @Override
    protected Class<?> getEntityType(Method method) {
        return method.getReturnType();
    }

    @Override
    public Object invoke(Object[] args) {
        Object result = super.invoke(args);
        if (result instanceof List) {
            List<?> resultList = (List<?>) result;
            if (resultList.size() > 0)
                return resultList.get(0);
        }
        return null;
    }
}
