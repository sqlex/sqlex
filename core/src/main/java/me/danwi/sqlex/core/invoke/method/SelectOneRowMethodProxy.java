package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;
import org.jetbrains.annotations.NotNull;

import java.beans.IntrospectionException;
import java.lang.reflect.Method;

public class SelectOneRowMethodProxy extends SelectMethodProxy{
    public SelectOneRowMethodProxy(@NotNull Method method, @NotNull TransactionManager transactionManager, @NotNull ParameterConverterRegistry registry) throws SqlExImpossibleException, IntrospectionException {
        super(method, transactionManager, registry);
    }
}
