package me.danwi.sqlex.core;

import me.danwi.sqlex.core.invoke.InvocationProxy;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Proxy;
import java.util.HashMap;
import java.util.Map;

public class DaoFactory {
    final private TransactionManager transactionManager;
    final private ParameterConverterRegistry parameterConverterRegistry;
    final private Map<Class<?>, InvocationProxy> invocationProxyCache = new HashMap<>();

    public DaoFactory(TransactionManager transactionManager, Class<? extends RepositoryLike> repository) throws Exception {
        this.transactionManager = transactionManager;
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
    }

    public <D> D getInstance(Class<D> dao) {
        //尝试从缓存中获取
        InvocationProxy invocationProxy = invocationProxyCache.get(dao);
        if (invocationProxy == null) {
            synchronized (invocationProxyCache) {
                invocationProxy = invocationProxyCache.get(dao);
                if (invocationProxy == null) {
                    //缓存中没有再自己新建
                    invocationProxy = new InvocationProxy(transactionManager, parameterConverterRegistry);
                    invocationProxyCache.put(dao, invocationProxy);
                }
            }
        }

        //noinspection unchecked
        return (D) Proxy.newProxyInstance(
                dao.getClassLoader(),
                new Class[]{dao},
                invocationProxy
        );
    }
}
