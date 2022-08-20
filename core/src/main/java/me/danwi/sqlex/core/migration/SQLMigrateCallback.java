package me.danwi.sqlex.core.migration;

import com.alibaba.druid.DbType;
import com.alibaba.druid.sql.SQLUtils;
import com.alibaba.druid.sql.ast.SQLStatement;

import java.util.List;

public class SQLMigrateCallback {
    public static MigrateCallback before(int version, String script) {
        return VersionMigrateCallback.before(version, executor -> {
            List<SQLStatement> statements = SQLUtils.parseStatements(script, DbType.mysql);
            for (SQLStatement statement : statements) {
                executor.execute(statement.toString());
            }
        });
    }

    public static MigrateCallback after(int version, String script) {
        return VersionMigrateCallback.after(version, executor -> {
            List<SQLStatement> statements = SQLUtils.parseStatements(script, DbType.mysql);
            for (SQLStatement statement : statements) {
                executor.execute(statement.toString());
            }
        });
    }
}
