package me.danwi.sqlex.core.exception;

/**
 * 未声明的Checked异常包装,通常出现在调用{@link me.danwi.sqlex.core.DaoFactory#transaction}时,action中抛出checked异常
 */
public class SqlExUndeclaredException extends SqlExException {
    public SqlExUndeclaredException(Throwable cause) {
        super(cause);
    }
}
