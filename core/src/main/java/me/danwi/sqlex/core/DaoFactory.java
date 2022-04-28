package me.danwi.sqlex.core;

import com.mysql.cj.jdbc.MysqlConnectionPoolDataSource;
import me.danwi.sqlex.core.annotation.SqlExRepository;
import me.danwi.sqlex.core.checker.Checker;
import me.danwi.sqlex.core.exception.SqlExRepositoryNotMatchException;
import me.danwi.sqlex.core.exception.SqlExSQLException;
import me.danwi.sqlex.core.exception.SqlExUndeclaredException;
import me.danwi.sqlex.core.invoke.InvocationProxy;
import me.danwi.sqlex.core.migration.Migrator;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.DefaultTransactionManager;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import javax.sql.DataSource;
import java.io.IOException;
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
    final private ExceptionTranslator exceptionTranslator;
    final private Migrator migrator;
    final private Checker checker;

    /**
     * 默认异常翻译
     *
     * <p>{@link SQLException}将转换成{@link me.danwi.sqlex.core.exception.SqlExSQLException}
     *
     * <p>{@link RuntimeException}分类异常保持不变
     *
     * <p>其他的Checked异常,将全部转换成{@link SqlExUndeclaredException}
     */
    static class DefaultExceptionTranslator implements ExceptionTranslator {
        @Override
        public RuntimeException translate(Exception ex) {
            if (ex instanceof SQLException) {
                return new SqlExSQLException((SQLException) ex);
            } else if (ex instanceof RuntimeException) {
                return (RuntimeException) ex;
            } else {
                return new SqlExUndeclaredException(ex);
            }
        }
    }

    /**
     * 新建数据访问对象工厂实例
     *
     * @param url        连接
     * @param username   用户名
     * @param password   密码
     * @param repository SqlEx Repository
     */
    public DaoFactory(String url, String username, String password, Class<? extends RepositoryLike> repository) {
        MysqlConnectionPoolDataSource dataSource = new MysqlConnectionPoolDataSource();
        dataSource.setURL(url);
        dataSource.setUser(username);
        dataSource.setPassword(password);
        this.repositoryClass = repository;
        this.exceptionTranslator = new DefaultExceptionTranslator();
        this.transactionManager = new DefaultTransactionManager(dataSource, this.exceptionTranslator);
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
        this.migrator = new Migrator(this);
        this.checker = new Checker(this);
    }

    /**
     * 新建数据访问对象工厂实例,使用默认事务管理器
     *
     * @param dataSource 数据源
     * @param repository SqlEx Repository
     */
    public DaoFactory(DataSource dataSource, Class<? extends RepositoryLike> repository) {
        this.repositoryClass = repository;
        this.exceptionTranslator = new DefaultExceptionTranslator();
        this.transactionManager = new DefaultTransactionManager(dataSource, this.exceptionTranslator);
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
        this.migrator = new Migrator(this);
        this.checker = new Checker(this);
    }

    /**
     * 使用指定的事务管理器来新建数据访问对象工厂实例
     *
     * @param transactionManager  事务管理器
     * @param repository          SqlEx Repository
     * @param exceptionTranslator 异常翻译
     */
    public DaoFactory(TransactionManager transactionManager, Class<? extends RepositoryLike> repository, ExceptionTranslator exceptionTranslator) {
        this.repositoryClass = repository;
        this.transactionManager = transactionManager;
        this.parameterConverterRegistry = ParameterConverterRegistry.fromRepository(repository);
        this.exceptionTranslator = exceptionTranslator;
        this.migrator = new Migrator(this);
        this.checker = new Checker(this);
    }

    /**
     * 新建事务,适合手动管理事务
     *
     * @return 事务
     */
    public Transaction newTransaction() {
        return this.transactionManager.newTransaction();
    }

    /**
     * 以事务的方式来运行函数
     * 使用默认事务隔离级别
     *
     * @param action 函数
     * @param <T>    闭包函数的返回值
     * @return 返回闭包函数的返回值
     * @throws SqlExUndeclaredException action运行过程中的Checked异常包装
     */
    public <T> T transaction(TransactionAction<T> action) {
        return transaction(transactionManager.getDefaultIsolationLevel(), action);
    }

    /**
     * 以事务的方式来运行函数,函数无返回值
     * 使用默认事务隔离级别
     *
     * @param action 函数
     * @throws SqlExUndeclaredException action运行过程中的Checked异常包装
     */
    public void transaction(TransactionActionReturnVoid action) {
        transaction(transactionManager.getDefaultIsolationLevel(), action);
    }

    /**
     * 以事务的方式来运行函数,函数无返回值
     *
     * @param transactionIsolationLevel 事务隔离级别, 例如:{@link Connection#TRANSACTION_REPEATABLE_READ}
     * @param action                    函数
     * @throws SqlExUndeclaredException action运行过程中的Checked异常包装
     */
    public void transaction(Integer transactionIsolationLevel, TransactionActionReturnVoid action) {
        transaction(transactionIsolationLevel, transaction -> {
            action.run(transaction);
            return null;
        });
    }

    /**
     * 以事务的方式来运行函数
     *
     * @param action                    函数
     * @param transactionIsolationLevel 事务隔离级别, 例如:{@link Connection#TRANSACTION_REPEATABLE_READ}
     * @param <T>                       闭包函数的返回值
     * @return 返回闭包函数的返回值
     * @throws SqlExUndeclaredException action运行过程中的Checked异常包装
     */
    public <T> T transaction(Integer transactionIsolationLevel, TransactionAction<T> action) {
        //获取当前的事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        //是否为顶级事务
        boolean isTopLevelTransaction = false;
        //如果当前不存在事务,则新建一个
        if (currentTransaction == null) {
            isTopLevelTransaction = true;
            currentTransaction = transactionManager.newTransaction(transactionIsolationLevel);
        }
        //异常
        RuntimeException exception = null;
        try {
            //运行并获取到结果
            return action.run(currentTransaction);
        } catch (Exception e) {
            //异常转换
            exception = this.exceptionTranslator.translate(e);
            throw exception;
        } finally {
            if (isTopLevelTransaction) {
                try {
                    if (exception == null)
                        currentTransaction.commit();
                    else
                        currentTransaction.rollback();
                } finally {
                    try {
                        currentTransaction.close();
                    } catch (IOException ignored) {
                        //忽略事务关闭异常
                    }
                }
            }
        }
    }

    /**
     * 新建连接
     *
     * @return 新建的数据连接
     */
    public Connection newConnection() {
        return this.transactionManager.newConnection();
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
     * 迁移数据版本
     */
    public void migrate() {
        migrator.migrate();
    }

    /**
     * 检查数据结构
     */
    public void check() {
        checker.check();
    }

    /**
     * 获取数据访问对象的实例
     *
     * @param dao 数据访问对象Class
     * @param <D> 数据访问对象类型
     * @return 数据访问对象实例
     * @throws SqlExRepositoryNotMatchException 给定的Dao接口不属于Factory管理的Repository
     */
    public <D> D getInstance(Class<D> dao) {
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
                    invocationProxy = new InvocationProxy(transactionManager, parameterConverterRegistry, exceptionTranslator);
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
