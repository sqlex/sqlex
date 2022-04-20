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
    public @NotNull Transaction newTransaction() throws SQLException {
        Connection connection = newConnection();
        DefaultTransaction defaultTransaction = new DefaultTransaction(connection);
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

        public DefaultTransaction(Connection connection) throws SQLException {
            this.connection = connection;
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
                if (originAutoCommit)
                    connection.setAutoCommit(true);
                connection.close();
            } catch (SQLException ignored) {
            } finally {
                threadLocal.set(null);
            }
        }
    }
}
