package me.danwi.sqlex.core;

import me.danwi.sqlex.core.invoke.InvocationProxy;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.DefaultTransactionManager;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import javax.sql.DataSource;
import java.lang.reflect.Proxy;
import java.sql.SQLException;
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

    public DaoFactory(DataSource dataSource, Class<? extends RepositoryLike> repository) throws Exception {
        this.transactionManager = new DefaultTransactionManager(dataSource);
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
    }

    public Transaction newTransaction() throws SQLException {
        return this.transactionManager.newTransaction();
    }
    
    public interface Action<T> {
        T run(Transaction transaction) throws Exception;
    }

    public <T> T transaction(Action<T> action) throws Exception {
        //获取当前的事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        //是否为顶级事务
        boolean isTopLevelTransaction = false;
        //如果当前不存在事务,则新建一个
        if (currentTransaction == null) {
            isTopLevelTransaction = true;
            currentTransaction = transactionManager.newTransaction();
        }

        try {
            T result = action.run(currentTransaction);
            if (isTopLevelTransaction)
                currentTransaction.commit();
            return result;
        } catch (Exception e) {
            if (isTopLevelTransaction)
                currentTransaction.rollback();
            throw e;
        } finally {
            if (isTopLevelTransaction)
                currentTransaction.close();
        }
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
