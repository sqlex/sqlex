package me.danwi.sqlex.core.transaction;

import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.sql.Connection;

/**
 * 事务管理器
 */
public interface TransactionManager {
    /**
     * 获取当前存在的事务
     *
     * @return 当前正在进行的事务, 没有则返回空
     */
    @Nullable Transaction getCurrentTransaction();

    /**
     * 新建事务
     *
     * @return 新建立的事务
     */
    @NotNull Transaction newTransaction();

    /**
     * 直接获取数据库连接(手动挡)
     *
     * @return 数据库连接
     */
    @NotNull Connection newConnection();
}
