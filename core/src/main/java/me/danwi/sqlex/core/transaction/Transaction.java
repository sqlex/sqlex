package me.danwi.sqlex.core.transaction;

import org.jetbrains.annotations.NotNull;

import java.io.Closeable;
import java.sql.Connection;

/**
 * 事务
 */
public interface Transaction extends Closeable {
    /**
     * 获取当前事务的数据库连接
     *
     * @return 数据库连接
     */
    @NotNull Connection getConnection();

    /**
     * 提交当前事务
     */
    void commit();

    /**
     * 回滚当前事务
     */
    void rollback();
}
