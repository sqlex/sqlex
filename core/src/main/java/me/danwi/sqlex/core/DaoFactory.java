package me.danwi.sqlex.core;

import com.mysql.cj.jdbc.MysqlConnectionPoolDataSource;
import me.danwi.sqlex.core.annotation.SqlExRepository;
import me.danwi.sqlex.core.exception.SqlExException;
import me.danwi.sqlex.core.exception.SqlExRepositoryNotMatchException;
import me.danwi.sqlex.core.invoke.InvocationProxy;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.DefaultTransactionManager;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import javax.sql.DataSource;
import java.lang.reflect.Proxy;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.HashMap;
import java.util.Map;

public class DaoFactory {
    final private Class<?> repositoryClass;
    final private TransactionManager transactionManager;
    final private ParameterConverterRegistry parameterConverterRegistry;
    final private Map<Class<?>, InvocationProxy> invocationProxyCache = new HashMap<>();

    /**
     * 新建数据访问对象工厂实例
     *
     * @param url        连接
     * @param username   用户名
     * @param password   密码
     * @param repository SqlEx Repository
     * @throws SQLException 转换器解析异常
     */
    public DaoFactory(String url, String username, String password, Class<? extends RepositoryLike> repository) throws SQLException {
        MysqlConnectionPoolDataSource dataSource = new MysqlConnectionPoolDataSource();
        dataSource.setURL(url);
        dataSource.setUser(username);
        dataSource.setPassword(password);
        this.repositoryClass = repository;
        this.transactionManager = new DefaultTransactionManager(dataSource);
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
    }


    /**
     * 新建数据访问对象工厂实例,使用默认事务管理器
     *
     * @param dataSource 数据源
     * @param repository SqlEx Repository
     * @throws SQLException 转换器解析异常
     */
    public DaoFactory(DataSource dataSource, Class<? extends RepositoryLike> repository) throws SQLException {
        this.repositoryClass = repository;
        this.transactionManager = new DefaultTransactionManager(dataSource);
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
    }

    /**
     * 使用指定的事务管理器来新建数据访问对象工厂实例
     *
     * @param transactionManager 事务管理器
     * @param repository         SqlEx Repository
     * @throws SQLException 转换器解析异常
     */
    public DaoFactory(TransactionManager transactionManager, Class<? extends RepositoryLike> repository) throws SQLException {
        this.repositoryClass = repository;
        this.transactionManager = transactionManager;
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
    }

    /**
     * 新建事务,适合手动管理事务
     *
     * @return 事务
     * @throws SQLException 新建事务异常
     */
    public Transaction newTransaction() throws SQLException {
        return this.transactionManager.newTransaction();
    }

    public interface Action<T> {
        T run(Transaction transaction) throws Exception;
    }

    /**
     * 以事务的方式来运行函数
     * 使用默认事务隔离级别
     *
     * @param action 函数
     * @param <T>    闭包函数的返回值
     * @return 返回闭包函数的返回值
     * @throws Exception action运行异常
     */
    public <T> T transaction(Action<T> action) throws Exception {
        return transaction(transactionManager.getDefaultIsolationLevel(), action);
    }

    /**
     * 以事务的方式来运行函数
     *
     * @param action                    函数
     * @param transactionIsolationLevel 事务隔离级别, 例如:{@link Connection#TRANSACTION_REPEATABLE_READ}
     * @param <T>                       闭包函数的返回值
     * @return 返回闭包函数的返回值
     * @throws Exception action运行异常
     */
    public <T> T transaction(int transactionIsolationLevel, Action<T> action) throws Exception {
        //获取当前的事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        //是否为顶级事务
        boolean isTopLevelTransaction = false;
        //如果当前不存在事务,则新建一个
        if (currentTransaction == null) {
            isTopLevelTransaction = true;
            currentTransaction = transactionManager.newTransaction(transactionIsolationLevel);
        }

        try {
            //运行并获取到结果
            T result = action.run(currentTransaction);
            if (isTopLevelTransaction)
                currentTransaction.commit();
            return result;
        } catch (Exception e) {
            //发生异常,如果是顶层事务,则需要回滚
            if (isTopLevelTransaction)
                currentTransaction.rollback();
            //继续向上抛出异常
            throw e;
        } finally {
            //如果是顶层事务,需要在最后关闭事务
            if (isTopLevelTransaction)
                currentTransaction.close();
        }
    }

    /**
     * 获取该Dao工厂做管理的SqlEx Repository
     *
     * @return SqlEx Repository
     */
    public Class<?> getRepositoryClass() {
        return this.repositoryClass;
    }

    /**
     * 获取数据访问对象的实例
     *
     * @param dao 数据访问对象Class
     * @param <D> 数据访问对象类型
     * @return 数据访问对象实例
     * @throws SqlExRepositoryNotMatchException 给定的Dao接口不属于Factory管理的Repository
     */
    public <D> D getInstance(Class<D> dao) throws SqlExException {
        //尝试从缓存中获取
        InvocationProxy invocationProxy = invocationProxyCache.get(dao);
        if (invocationProxy == null) {
            synchronized (invocationProxyCache) {
                invocationProxy = invocationProxyCache.get(dao);
                if (invocationProxy == null) {
                    //检查这个Dao接口是否属于repository
                    SqlExRepository annotation = dao.getAnnotation(SqlExRepository.class);
                    if (annotation == null)
                        throw new SqlExRepositoryNotMatchException();
                    if (!annotation.value().getName().equals(this.repositoryClass.getName()))
                        throw new SqlExRepositoryNotMatchException();
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
