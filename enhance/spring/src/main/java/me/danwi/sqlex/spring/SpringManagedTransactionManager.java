package me.danwi.sqlex.spring;

import me.danwi.sqlex.core.ExceptionTranslator;
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
    private final ExceptionTranslator translator;

    public SpringManagedTransactionManager(DataSource dataSource, ExceptionTranslator translator) {
        if (dataSource instanceof TransactionAwareDataSourceProxy) {
            this.dataSource = dataSource;
        } else {
            this.dataSource = new TransactionAwareDataSourceProxy(dataSource);
        }
        this.translator = translator;
    }

    @Override
    public Integer getDefaultIsolationLevel() {
        return null;
    }

    @Override
    public Transaction getCurrentTransaction() {
        //直接返回空,对于调用者来说,相当于没有事务(实际上由spring接管),会使用new connection来处理(从而获得事务连接)
        return null;
    }

    @Override
    public Transaction newTransaction(Integer transactionIsolationLevel) {
        throw new UnsupportedOperationException("事务已经被spring托管,请使用spring框架对应的API来声明事务");
    }

    @Override
    public Connection newConnection() {
        try {
            return dataSource.getConnection();
        } catch (SQLException e) {
            throw translator.translate(e);
        }
    }
}
