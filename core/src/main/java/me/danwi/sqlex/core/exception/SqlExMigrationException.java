package me.danwi.sqlex.core.exception;

/**
 * 版本迁移异常
 */
public class SqlExMigrationException extends SqlExException {
    private int version = -1;

    public SqlExMigrationException(String message) {
        super(message);
    }

    public SqlExMigrationException(int version, Exception cause) {
        super(cause);
        this.version = version;
    }

    public int getVersion() {
        return version;
    }
}
