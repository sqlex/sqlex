package me.danwi.sqlex.core.logging;


import me.danwi.sqlex.core.exception.SqlExException;

/**
 * SqlEx日志异常
 */
public class LogException extends SqlExException {

    private static final long serialVersionUID = -1644142198794818725L;

    public LogException() {
        super();
    }

    public LogException(String message) {
        super(message);
    }

    public LogException(String message, Throwable cause) {
        super(message, cause);
    }

    public LogException(Throwable cause) {
        super(cause);
    }

}
