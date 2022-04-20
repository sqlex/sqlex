package me.danwi.sqlex.core.transaction;

import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import javax.sql.DataSource;
import java.io.IOException;
import java.sql.Connection;
import java.sql.SQLException;

/**
 * 默认的基于ThreadLocal的事务管理器
 */
public class DefaultTransactionManager implements TransactionManager {
    final private DataSource dataSource;

    final private ThreadLocal<Transaction> threadLocal = new ThreadLocal<>();


    public DefaultTransactionManager(DataSource dataSource) {
        this.dataSource = dataSource;
    }

    @Override
    public @Nullable Transaction getCurrentTransaction() {
        return threadLocal.get();
    }

    @Override
    public @NotNull Transaction newTransaction(int transactionIsolationLevel) throws SQLException {
        Connection connection = newConnection();
        DefaultTransaction defaultTransaction = new DefaultTransaction(connection, transactionIsolationLevel);
        threadLocal.set(defaultTransaction);
        return defaultTransaction;
    }

    @Override
    public @NotNull Connection newConnection() throws SQLException {
        return dataSource.getConnection();
    }

    /**
     * 默认事务
     */
    public class DefaultTransaction implements Transaction {
        final private Connection connection;
        private final boolean originAutoCommit;
        private final int originIsolationLevel;

        public DefaultTransaction(Connection connection, int desiredIsolationLevel) throws SQLException {
            this.connection = connection;
            //设置自动提交
            originAutoCommit = connection.getAutoCommit();
            if (originAutoCommit)
                connection.setAutoCommit(false);
            //设置事务隔离级别
            originIsolationLevel = connection.getTransactionIsolation();
            if (originIsolationLevel != desiredIsolationLevel)
                connection.setTransactionIsolation(desiredIsolationLevel);
        }

        @Override
        public @NotNull Connection getConnection() {
            return connection;
        }

        @Override
        public void commit() throws SQLException {
            connection.commit();
        }

        @Override
        public void rollback() throws SQLException {
            connection.rollback();
        }

        @Override
        public void close() throws IOException {
            try {
                //如果已经关闭来
                if (connection.isClosed())
                    return;
                //还原自动提交属性
                if (originAutoCommit)
                    connection.setAutoCommit(true);
                //还原事务隔离级别属性
                if (originIsolationLevel != connection.getTransactionIsolation())
                    connection.setTransactionIsolation(originIsolationLevel);
                //关闭连接
                connection.close();
            } catch (SQLException ignored) {
            } finally {
                threadLocal.remove();
            }
        }
    }
}
