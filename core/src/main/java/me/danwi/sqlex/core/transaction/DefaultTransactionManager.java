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

    private Integer defaultIsolationLevel;

    final private ThreadLocal<Transaction> threadLocal = new ThreadLocal<>();


    public DefaultTransactionManager(DataSource dataSource) {
        this.dataSource = dataSource;
    }

    public DefaultTransactionManager(DataSource dataSource, int defaultIsolationLevel) {
        this.dataSource = dataSource;
        this.defaultIsolationLevel = defaultIsolationLevel;
    }

    @Override
    public Integer getDefaultIsolationLevel() {
        return defaultIsolationLevel;
    }

    @Override
    public @Nullable Transaction getCurrentTransaction() {
        return threadLocal.get();
    }

    @Override
    public @NotNull Transaction newTransaction(Integer transactionIsolationLevel) throws SQLException {
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
        private final Connection connection;
        private final boolean originAutoCommit;
        private Integer originIsolationLevel = Integer.MIN_VALUE;
        private final Integer desiredIsolationLevel;

        public DefaultTransaction(Connection connection, Integer desiredIsolationLevel) throws SQLException {
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
