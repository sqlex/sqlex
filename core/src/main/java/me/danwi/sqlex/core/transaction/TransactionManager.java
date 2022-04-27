package me.danwi.sqlex.core.transaction;

import java.sql.Connection;

/**
 * 事务管理器
 */
public interface TransactionManager {
    /**
     * 获取默认事务管理级别
     *
     * @return 默认事务管理级别
     */
    Integer getDefaultIsolationLevel();

    /**
     * 获取当前存在的事务
     *
     * @return 当前正在进行的事务, 没有则返回空
     */
    Transaction getCurrentTransaction();

    /**
     * 新建事务
     * 使用默认事务隔离级别
     *
     * @return 新建立的事务
     */
    default Transaction newTransaction() {
        return newTransaction(getDefaultIsolationLevel());
    }

    /**
     * 新建事务
     *
     * @param transactionIsolationLevel 事务隔离级别
     * @return 新建立的事务
     */
    Transaction newTransaction(Integer transactionIsolationLevel);

    /**
     * 直接获取数据库连接(手动挡)
     *
     * @return 数据库连接
     */
    Connection newConnection();
}
