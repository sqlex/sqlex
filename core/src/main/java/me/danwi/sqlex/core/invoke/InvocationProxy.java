package me.danwi.sqlex.core.invoke;

import me.danwi.sqlex.core.repository.ParameterSetter;
import me.danwi.sqlex.core.repository.ResultGetter;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;

public class InvocationProxy implements InvocationHandler {
    final private TransactionManager transactionManager;
    final private ParameterSetter parameterSetter;
    final private ResultGetter resultGetter;

    public InvocationProxy(TransactionManager transactionManager, ParameterSetter parameterSetter, ResultGetter resultGetter) {
        this.transactionManager = transactionManager;
        this.parameterSetter = parameterSetter;
        this.resultGetter = resultGetter;
    }

    @Override
    public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
        return null;
    }
}
