package me.danwi.sqlex.core.invoke;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.annotation.method.SqlExOneRow;
import me.danwi.sqlex.core.annotation.method.SqlExPaged;
import me.danwi.sqlex.core.annotation.method.type.SqlExDelete;
import me.danwi.sqlex.core.annotation.method.type.SqlExInsert;
import me.danwi.sqlex.core.annotation.method.type.SqlExSelect;
import me.danwi.sqlex.core.annotation.method.type.SqlExUpdate;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.invoke.method.*;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.Map;

public class InvocationProxy implements InvocationHandler {
    final private TransactionManager transactionManager;
    final private ParameterSetter parameterSetter;
    final private ExceptionTranslator translator;

    final private Map<Method, MethodProxy> methodProxyCache = new HashMap<>();

    public InvocationProxy(TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        this.transactionManager = transactionManager;
        this.parameterSetter = parameterSetter;
        this.translator = translator;
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
                            methodProxy = new SelectPagedMethodProxy(method, transactionManager, parameterSetter, translator);
                        } else if (method.getDeclaredAnnotation(SqlExOneRow.class) != null) {
                            methodProxy = new SelectOneRowMethodProxy(method, transactionManager, parameterSetter, translator);
                        } else {
                            methodProxy = new SelectMethodProxy(method, transactionManager, parameterSetter, translator);
                        }
                    } else if (method.getAnnotation(SqlExInsert.class) != null) {
                        methodProxy = new InsertMethodProxy(method, transactionManager, parameterSetter, translator);
                    } else if (method.getAnnotation(SqlExUpdate.class) != null
                            || method.getAnnotation(SqlExDelete.class) != null) {
                        methodProxy = new UpdateDeleteMethodProxy(method, transactionManager, parameterSetter, translator);
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
