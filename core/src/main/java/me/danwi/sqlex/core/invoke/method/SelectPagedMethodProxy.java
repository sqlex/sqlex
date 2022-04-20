package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;
import org.jetbrains.annotations.NotNull;

import java.lang.reflect.Method;

public class SelectPagedMethodProxy extends SelectMethodProxy {
    public SelectPagedMethodProxy(@NotNull Method method, @NotNull TransactionManager transactionManager, @NotNull ParameterConverterRegistry registry) throws SqlExImpossibleException {
        super(method, transactionManager, registry);
    }
}
