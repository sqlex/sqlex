package me.danwi.sqlex.core.migration;

import com.alibaba.druid.DbType;
import com.alibaba.druid.sql.SQLUtils;
import com.alibaba.druid.sql.ast.SQLStatement;
import me.danwi.sqlex.core.exception.SqlExException;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.net.MalformedURLException;
import java.net.URL;
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
        return before(version, readToString(inputStream));
    }

    public static MigrateCallback after(int version, InputStream inputStream) {
        String script;
        script = readToString(inputStream);
        return after(version, script);
    }


    public static MigrateCallback before(int version, URL url) {
        try (InputStream inputStream = url.openStream()) {
            return before(version, inputStream);
        } catch (IOException e) {
            throw new SqlExException("从URL中获取版本迁移回调脚本文件流异常", e);
        }
    }

    public static MigrateCallback after(int version, URL url) {
        try (InputStream inputStream = url.openStream()) {
            return after(version, inputStream);
        } catch (IOException e) {
            throw new SqlExException("从URL中获取版本迁移回调脚本文件流异常", e);
        }
    }

    public static MigrateCallback before(int version, File file) {
        return before(version, toScriptUrl(file));
    }


    public static MigrateCallback after(int version, File file) {
        return after(version, toScriptUrl(file));
    }

    public static MigrateCallback beforeForPath(int version, String path) {
        return before(version, toScriptUrl(new File(path)));
    }


    public static MigrateCallback afterForPath(int version, String path) {
        return after(version, toScriptUrl(new File(path)));
    }


    private static URL toScriptUrl(File file) {
        if (file.isDirectory()) {
            throw new SqlExException("该文件是文件夹，无法转换为版本迁移回调脚本");
        }
        if (!file.exists()) {
            throw new SqlExException("该文件不存在，无法转换为版本迁移回调脚本");
        }
        try {
            return file.toURI().toURL();
        } catch (MalformedURLException e) {
            throw new SqlExException("版本迁移回调脚本文件转换URL异常");
        }
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
            throw new SqlExException("版本迁移回调脚本文件转换字符串异常", e);
        }
    }
}
