package me.danwi.sqlex.core.transaction;

import me.danwi.sqlex.core.ExceptionTranslator;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.SQLException;

/**
 * 默认的基于ThreadLocal的事务管理器
 */
public class DefaultTransactionManager implements TransactionManager {
    final private DataSource dataSource;
    final private ExceptionTranslator exceptionTranslator;
    private Integer defaultIsolationLevel;

    final private ThreadLocal<Transaction> threadLocal = new ThreadLocal<>();

    public DefaultTransactionManager(DataSource dataSource, ExceptionTranslator translator) {
        this.dataSource = dataSource;
        this.exceptionTranslator = translator;
    }

    public DefaultTransactionManager(DataSource dataSource, int defaultIsolationLevel, ExceptionTranslator translator) {
        this.dataSource = dataSource;
        this.defaultIsolationLevel = defaultIsolationLevel;
        this.exceptionTranslator = translator;
    }

    @Override
    public Integer getDefaultIsolationLevel() {
        return defaultIsolationLevel;
    }

    @Override
    public Transaction getCurrentTransaction() {
        return threadLocal.get();
    }

    @Override
    public Transaction newTransaction(Integer transactionIsolationLevel) {
        Connection connection = newConnection();
        DefaultTransaction defaultTransaction = new DefaultTransaction(connection, transactionIsolationLevel);
        threadLocal.set(defaultTransaction);
        return defaultTransaction;
    }

    @Override
    public Connection newConnection() {
        try {
            return dataSource.getConnection();
        } catch (SQLException e) {
            throw exceptionTranslator.translate(e);
        }
    }

    /**
     * 默认事务
     */
    public class DefaultTransaction implements Transaction {
        private final Connection connection;
        private final boolean originAutoCommit;
        private Integer originIsolationLevel = Integer.MIN_VALUE;
        private final Integer desiredIsolationLevel;

        public DefaultTransaction(Connection connection, Integer desiredIsolationLevel) {
            try {
                this.connection = connection;
                this.desiredIsolationLevel = desiredIsolationLevel;
                //设置事务隔离级别
                if (desiredIsolationLevel != null) {
                    originIsolationLevel = connection.getTransactionIsolation();
                    if (!originIsolationLevel.equals(desiredIsolationLevel))
                        connection.setTransactionIsolation(desiredIsolationLevel);
                }
                //设置自动提交
                originAutoCommit = connection.getAutoCommit();
                if (originAutoCommit)
                    connection.setAutoCommit(false);
            } catch (SQLException e) {
                throw exceptionTranslator.translate(e);
            }
        }

        @Override
        public Connection getConnection() {
            return connection;
        }

        @Override
        public void commit() {
            try {
                connection.commit();
            } catch (SQLException e) {
                throw exceptionTranslator.translate(e);
            }
        }

        @Override
        public void rollback() {
            try {
                connection.rollback();
            } catch (SQLException e) {
                throw exceptionTranslator.translate(e);
            }
        }

        @Override
        public void close() {
            try {
                //如果已经关闭来
                if (connection.isClosed())
                    return;
                //还原事务隔离级别属性
                if (desiredIsolationLevel != null && !originIsolationLevel.equals(desiredIsolationLevel))
                    connection.setTransactionIsolation(originIsolationLevel);
                //还原自动提交属性
                if (originAutoCommit)
                    connection.setAutoCommit(true);
                //关闭连接
                connection.close();
            } catch (SQLException ignored) {
            } finally {
                threadLocal.remove();
            }
        }
    }
}
