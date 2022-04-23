package me.danwi.sqlex.core.exception;

/**
 * 不可能出现的异常,由于sqlex在编译时保证了各种情况的确定性,有些分支条件下是不可能出现异常的.
 */
public class SqlExImpossibleException extends SqlExBaseException {

    public SqlExImpossibleException() {
        super();
    }

    public SqlExImpossibleException(String message) {
        super(message);
    }

    public SqlExImpossibleException(String message, Throwable cause) {
        super(message, cause);
    }

    public SqlExImpossibleException(Throwable cause) {
        super(cause);
    }
}
