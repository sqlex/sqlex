package me.danwi.sqlex.core.invoke;

import me.danwi.sqlex.core.annotation.method.SqlExOneRow;
import me.danwi.sqlex.core.annotation.method.SqlExPaged;
import me.danwi.sqlex.core.annotation.method.type.SqlExDelete;
import me.danwi.sqlex.core.annotation.method.type.SqlExInsert;
import me.danwi.sqlex.core.annotation.method.type.SqlExSelect;
import me.danwi.sqlex.core.annotation.method.type.SqlExUpdate;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.invoke.method.*;
import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.Map;

public class InvocationProxy implements InvocationHandler {
    private final RawSQLExecutor executor;
    private final Map<Method, MethodProxy> methodProxyCache = new HashMap<>();

    public InvocationProxy(RawSQLExecutor executor) {
        this.executor = executor;
    }

    @Override
    public Object invoke(Object proxy, Method method, Object[] args) {
        //尝试从缓存中获取
        MethodProxy methodProxy = methodProxyCache.get(method);
        if (methodProxy == null) {
            synchronized (methodProxyCache) {
                methodProxy = methodProxyCache.get(method);
                if (methodProxy == null) {
                    //解析他的类型,生成不同的method proxy
                    if (method.getDeclaredAnnotation(SqlExSelect.class) != null) {
                        if (method.getDeclaredAnnotation(SqlExPaged.class) != null) {
                            methodProxy = new SelectPagedMethodProxy(method, this.executor);
                        } else if (method.getDeclaredAnnotation(SqlExOneRow.class) != null) {
                            methodProxy = new SelectOneRowMethodProxy(method, this.executor);
                        } else {
                            methodProxy = new SelectMethodProxy(method, this.executor);
                        }
                    } else if (method.getAnnotation(SqlExInsert.class) != null) {
                        methodProxy = new InsertMethodProxy(method, this.executor);
                    } else if (method.getAnnotation(SqlExUpdate.class) != null
                            || method.getAnnotation(SqlExDelete.class) != null) {
                        methodProxy = new UpdateDeleteMethodProxy(method, this.executor);
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
