package me.danwi.sqlex.core.exception;

/**
 * SqlEx 核心包 异常基类
 */
public class SqlExException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    public SqlExException() {
        super();
    }

    public SqlExException(String message) {
        super(message);
    }

    public SqlExException(String message, Throwable cause) {
        super(message, cause);
    }

    public SqlExException(Throwable cause) {
        super(cause);
    }
}
