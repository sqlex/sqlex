package me.danwi.sqlex.core;

import me.danwi.sqlex.core.invoke.InvocationProxy;
import me.danwi.sqlex.core.repository.ParameterSetter;
import me.danwi.sqlex.core.repository.ResultGetter;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Proxy;

public class DaoFactory {
    final private TransactionManager transactionManager;
    final private ParameterSetter parameterSetter;
    final private ResultGetter resultGetter;

    public DaoFactory(TransactionManager transactionManager, Class<? extends RepositoryLike> repository) throws Exception {
        this.transactionManager = transactionManager;
        this.parameterSetter = new ParameterSetter(repository);
        this.resultGetter = new ResultGetter(repository);
    }

    public <D> D getInstance(Class<D> dao) {
        InvocationProxy handler = new InvocationProxy(transactionManager, parameterSetter, resultGetter);
        //noinspection unchecked
        return (D) Proxy.newProxyInstance(
                dao.getClassLoader(),
                new Class[]{dao},
                handler);
    }
}
