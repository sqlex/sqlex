package me.danwi.sqlex.core.transaction;

import java.io.Closeable;
import java.sql.Connection;
import java.sql.SQLException;

/**
 * 事务
 */
public interface Transaction extends Closeable {
    /**
     * 获取当前事务的数据库连接
     *
     * @return 数据库连接
     */
    Connection getConnection();

    /**
     * 提交当前事务
     *
     * @throws SQLException 提交事务异常
     */
    void commit() throws SQLException;

    /**
     * 回滚当前事务
     *
     * @throws SQLException 提交事务异常
     */
    void rollback() throws SQLException;
}
