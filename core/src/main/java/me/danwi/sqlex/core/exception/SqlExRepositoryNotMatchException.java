package me.danwi.sqlex.core.exception;

/**
 * Dao接口不属于DaoFactory所管理的SqlEx Repository
 */
public class SqlExRepositoryNotMatchException extends SqlExException {
    public SqlExRepositoryNotMatchException() {
        super("Dao接口不属于DaoFactory所管理的SqlEx Repository");
    }
}
