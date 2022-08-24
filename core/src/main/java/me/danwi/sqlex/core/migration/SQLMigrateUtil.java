package me.danwi.sqlex.core.migration;

import com.alibaba.druid.DbType;
import com.alibaba.druid.sql.SQLUtils;
import com.alibaba.druid.sql.ast.SQLStatement;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.List;

public class SQLMigrateUtil {
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

    public static MigrateCallback before(int version, InputStream inputStream) {
        String script;
        script = readToString(inputStream);
        if (script != null) {
            return before(version, script);
        }
        return null;
    }

    public static MigrateCallback after(int version, InputStream inputStream) {
        String script;
        script = readToString(inputStream);
        if (script != null) {
            return after(version, script);
        }
        return null;
    }

    private static String readToString(InputStream inputStream) {
        try (ByteArrayOutputStream result = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[1024];
            int length;
            while ((length = inputStream.read(buffer)) != -1) {
                result.write(buffer, 0, length);
            }
            return result.toString("UTF-8");
        } catch (Exception e) {
            e.printStackTrace();
            return null;
        } finally {
            try {
                inputStream.close();
            } catch (IOException e) {
                e.printStackTrace();
            }
        }
    }
}
