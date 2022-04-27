package me.danwi.sqlex.core.exception;

import java.sql.SQLException;

/**
 * 由{@link SQLException}引发的异常
 */
public class SqlExSQLException extends SqlExException {
    public SqlExSQLException(SQLException cause) {
        super(cause);
    }
}
