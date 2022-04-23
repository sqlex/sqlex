package me.danwi.sqlex.core.exception;

/**
 * SqlEx 核心包 异常基类
 */
public class SqlExBaseException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    public SqlExBaseException() {
        super();
    }

    public SqlExBaseException(String message) {
        super(message);
    }

    public SqlExBaseException(String message, Throwable cause) {
        super(message, cause);
    }

    public SqlExBaseException(Throwable cause) {
        super(cause);
    }
}
