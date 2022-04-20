package me.danwi.sqlex.core.exception;

import java.sql.SQLException;

/**
 * 不可能出现的异常,由于sqlex在编译时保证了各种情况的确定性,有些分支条件下是不可能出现异常的.
 */
public class SqlExImpossibleException extends SQLException {
    public SqlExImpossibleException() {
        super();
    }

    public SqlExImpossibleException(String reason) {
        super(reason);
    }
}
