package me.danwi.sqlex.core.exception;

import me.danwi.sqlex.core.checker.TableInfo;

import java.util.ArrayList;
import java.util.List;

/**
 * 数据库检查异常
 */
public class SqlExCheckException extends SqlExException {
    private List<TableInfo> tables = new ArrayList<>();

    public SqlExCheckException(String message, Exception cause) {
        super(message, cause);
    }

    public SqlExCheckException(List<TableInfo> tables) {
        super("数据库与SqlEx Repository不一致");
        this.tables = tables;
    }

    public List<TableInfo> getTables() {
        return tables;
    }
}
