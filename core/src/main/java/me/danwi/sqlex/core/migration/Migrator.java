package me.danwi.sqlex.core.migration;

import me.danwi.sqlex.core.RepositoryLike;
import me.danwi.sqlex.core.annotation.SqlExSchema;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.exception.SqlExMigrationException;

import javax.sql.DataSource;
import java.sql.*;

public class Migrator {
    //数据源
    private final DataSource dataSource;
    //迁移任务定义
    private final Migration[] migrations;

    public Migrator(Class<? extends RepositoryLike> repository, DataSource dataSource) {
        SqlExSchema[] schemas = repository.getAnnotationsByType(SqlExSchema.class);
        migrations = new Migration[schemas.length];

        for (int i = 0; i < schemas.length; i++) {
            SqlExSchema schema = schemas[i];
            int version = schema.version();
            if (version >= schemas.length)
                throw new SqlExImpossibleException("版本号必须从0开始,且连续");
            if (migrations[version] != null)
                throw new SqlExImpossibleException("版本号重复,版本" + i + "已经存在");
            migrations[version] = new Migration(version, schema.scripts());
        }

        this.dataSource = dataSource;
    }

    //助手方法,执行语句
    private void execute(Connection connection, String sql) {
        try (Statement statement = connection.createStatement()) {
            statement.execute(sql);
        } catch (SQLException ex) {
            throw new SqlExMigrationException(-1, ex);
        }
    }

    //助手方法,执行语句
    private void execute(String sql) {
        try (Connection connection = dataSource.getConnection()) {
            execute(connection, sql);
        } catch (SQLException ex) {
            throw new SqlExMigrationException(-1, ex);
        }
    }

    /**
     * 迁移到最近版本
     *
     * @return 返回成功迁移的版本号
     */
    public int migrate() {
        return migrate(migrations.length - 1);
    }

    /**
     * 迁移到指定版本
     *
     * @param version 版本号
     * @return 返回成功迁移的版本号
     */
    public int migrate(int version) {
        //保证版本表的存在
        execute("create table if not exists _sqlex_version_(version int not null, can_migrate bool not null)");

        //专用于锁定版本信息的连接
        Connection lockConnection;
        //连接原本的自动提交属性
        boolean originAutoCommit = false;

        try {
            //获取连接
            lockConnection = dataSource.getConnection();
            //设置连接属性
            if (lockConnection.getAutoCommit()) {
                lockConnection.setAutoCommit(false);
                originAutoCommit = true;
            }
        } catch (SQLException ex) {
            throw new SqlExMigrationException(-1, ex);
        }

        try {
            //判断是否存在版本信息
            boolean hasVersion = false;
            int migratedVersion = -1;
            boolean canMigrate = false;
            try (PreparedStatement getVersionStatement = lockConnection.prepareStatement("select version,can_migrate from _sqlex_version_ for update")) {
                try (ResultSet resultSet = getVersionStatement.executeQuery()) {
                    if (resultSet.next()) {
                        //存在版本信息
                        hasVersion = true;
                        migratedVersion = resultSet.getInt(1);
                        canMigrate = resultSet.getBoolean(2);
                    }
                }
            }
            //不存在版本信息,插入版本信息
            if (!hasVersion) {
                execute(lockConnection, "insert into _sqlex_version_ values(-1, true)");
                canMigrate = true;
            }
            //如果无法迁移
            if (!canMigrate)
                throw new SqlExMigrationException("当前状态为无法执行迁移,可能是上次的迁移没有成功完成,需要人工介入");

            //开始迁移
            //版本号相等,无需迁移
            if (version == migratedVersion)
                return migratedVersion;
            //检查版本范围
            if (version <= migratedVersion || version >= migrations.length)
                throw new SqlExMigrationException("错误的版本号,当前版本范围" + migratedVersion + "<version<=" + (migrations.length - 1));
            //修改迁移状态
            execute(lockConnection, "update _sqlex_version_ set can_migrate=false");
            //挨个版本迁移
            for (int currentVersion = migratedVersion + 1; currentVersion <= version; currentVersion++) {
                Migration migration = migrations[currentVersion];
                String[] sqls = migration.getScripts();
                for (String sql : sqls) {
                    doMigrate(currentVersion, sql);
                }
            }
            //迁移完成,更新版本号,更新状态信息
            execute(lockConnection, "update _sqlex_version_ set can_migrate=true,version=" + version);

            //返回版本号
            return version;
        } catch (Exception ex) {
            //迁移过程中出现异常,记录状态信息
            execute(lockConnection, "update _sqlex_version_ set can_migrate=false");
            //包装异常
            if (ex instanceof SqlExMigrationException)
                throw (SqlExMigrationException) ex;
            else
                throw new SqlExMigrationException(-1, ex);
        } finally {
            try {
                //提交
                lockConnection.commit();
                //还原原本的自动提交属性
                if (originAutoCommit)
                    lockConnection.setAutoCommit(false);
                lockConnection.close();
            } catch (SQLException ignored) {
            }
        }
    }

    private void doMigrate(int version, String sql) {
        //获取连接
        try (Connection connection = dataSource.getConnection()) {
            //执行语句
            execute(connection, sql);
        } catch (SQLException e) {
            throw new SqlExMigrationException(version, e);
        }
    }
}
