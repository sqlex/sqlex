package me.danwi.sqlex.spring;

import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;
import org.springframework.jdbc.datasource.TransactionAwareDataSourceProxy;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.SQLException;

/**
 * 由spring框架托管的事务管理器
 */
public class SpringManagedTransactionManager implements TransactionManager {
    private final DataSource dataSource;

    public SpringManagedTransactionManager(DataSource dataSource) {
        if (dataSource instanceof TransactionAwareDataSourceProxy) {
            this.dataSource = dataSource;
        } else {
            this.dataSource = new TransactionAwareDataSourceProxy(dataSource);
        }
    }

    @Override
    public Integer getDefaultIsolationLevel() {
        return null;
    }

    @Override
    public Transaction getCurrentTransaction() {
        return null;
    }

    @Override
    public Transaction newTransaction(Integer transactionIsolationLevel) {
        throw new UnsupportedOperationException("事务已经被spring托管,请使用spring框架对应的API来声明事务");
    }

    @Override
    public Connection newConnection() throws SQLException {
        return dataSource.getConnection();
    }
}
