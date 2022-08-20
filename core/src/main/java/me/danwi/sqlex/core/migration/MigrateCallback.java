package me.danwi.sqlex.core.migration;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

/**
 * 迁移回调
 */
public interface MigrateCallback {
    /**
     * 迁移前调用
     *
     * @param version  准备迁移的版本
     * @param executor 原生SQL执行器
     * @throws Exception 迁移异常
     */
    void before(int version, RawSQLExecutor executor) throws Exception;

    /**
     * 迁移后调用
     *
     * @param version  迁移完成的版本
     * @param executor 原生SQL执行器
     * @throws Exception 迁移异常
     */
    void after(int version, RawSQLExecutor executor) throws Exception;
}
