package me.danwi.sqlex.core.exception;

import me.danwi.sqlex.core.checker.TableInfo;

import java.util.ArrayList;
import java.util.List;

/**
 * 数据库检查异常
 */
public class SqlExCheckException extends SqlExException {
    private final List<TableInfo> tables;

    public SqlExCheckException(Exception cause) {
        super(cause);
        this.tables = new ArrayList<>();
    }

    public SqlExCheckException(List<TableInfo> tables) {
        this.tables = tables;
    }

    public List<TableInfo> getTables() {
        return tables;
    }
}
