package me.danwi.sqlex.core.invoke;

import me.danwi.sqlex.core.annotation.*;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.invoke.method.*;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.sql.SQLException;
import java.util.HashMap;
import java.util.Map;

public class InvocationProxy implements InvocationHandler {
    final private TransactionManager transactionManager;
    final private ParameterConverterRegistry parameterConverterRegistry;

    final private Map<Method, MethodProxy> methodProxyCache = new HashMap<>();

    public InvocationProxy(TransactionManager transactionManager, ParameterConverterRegistry parameterConverterRegistry) {
        this.transactionManager = transactionManager;
        this.parameterConverterRegistry = parameterConverterRegistry;
    }

    @Override
    public Object invoke(Object proxy, Method method, Object[] args) throws SQLException {
        //尝试从缓存中获取
        MethodProxy methodProxy = methodProxyCache.get(method);
        if (methodProxy == null) {
            synchronized (methodProxyCache) {
                methodProxy = methodProxyCache.get(method);
                if (methodProxy == null) {
                    //解析他的类型,生成不同的method proxy
                    if (method.getDeclaredAnnotation(SqlExSelect.class) != null) {
                        if (method.getDeclaredAnnotation(SqlExPaged.class) != null) {
                            methodProxy = new SelectPagedMethodProxy(method, transactionManager, parameterConverterRegistry);
                        } else if (method.getDeclaredAnnotation(SqlExOneRow.class) != null) {
                            methodProxy = new SelectOneRowMethodProxy(method, transactionManager, parameterConverterRegistry);
                        } else {
                            methodProxy = new SelectMethodProxy(method, transactionManager, parameterConverterRegistry);
                        }
                    } else if (method.getAnnotation(SqlExInsert.class) != null) {
                        methodProxy = new InsertMethodProxy(method, transactionManager, parameterConverterRegistry);
                    } else if (method.getAnnotation(SqlExUpdate.class) != null
                            || method.getAnnotation(SqlExDelete.class) != null) {
                        methodProxy = new UpdateDeleteMethodProxy(method, transactionManager, parameterConverterRegistry);
                    } else {
                        throw new SqlExImpossibleException("错误的方法类型");
                    }
                    methodProxyCache.put(method, methodProxy);
                }
            }
        }

        return methodProxy.invoke(args);
    }
}
