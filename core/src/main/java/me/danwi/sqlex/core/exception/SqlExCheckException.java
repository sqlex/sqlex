package me.danwi.sqlex.core.exception;

/**
 * 真实数据库 与 SqlEx Repository 不一致
 */
public class SqlExCheckException extends SqlExException {
    public SqlExCheckException() {
        super("真实数据库 与 SqlEx Repository 不一致");
    }

    public SqlExCheckException(String message) {
        super(message);
    }

    public SqlExCheckException(String message, Exception cause) {
        super(message, cause);
    }
}
