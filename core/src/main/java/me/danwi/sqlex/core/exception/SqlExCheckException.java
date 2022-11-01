package me.danwi.sqlex.core.exception;

import me.danwi.sqlex.core.checker.TableInfo;

import java.util.List;

/**
 * 数据库检查异常
 */
public class SqlExCheckException extends SqlExException {
    private final List<TableInfo> missed;

    public SqlExCheckException(List<TableInfo> tables) {
        super("目标数据库与SqlEx Repository结构定义不一致");
        this.missed = tables;
    }

    /**
     * 目标数据库缺失的表及字段
     *
     * @return 表和字段
     */
    public List<TableInfo> getMissed() {
        return missed;
    }
}
