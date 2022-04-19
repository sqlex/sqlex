package me.danwi.sqlex.core.invoke;

import me.danwi.sqlex.core.invoke.getter.BeanResultGetter;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.sql.Connection;
import java.sql.PreparedStatement;

public class InvocationProxy implements InvocationHandler {
    final private TransactionManager transactionManager;
    final private ParameterConverterRegistry parameterConverterRegistry;

    public InvocationProxy(TransactionManager transactionManager, ParameterConverterRegistry parameterConverterRegistry) {
        this.transactionManager = transactionManager;
        this.parameterConverterRegistry = parameterConverterRegistry;
    }

    @Override
    public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
        return null;
    }
}
