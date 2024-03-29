package me.danwi.sqlex.core.migration;

import me.danwi.sqlex.core.DaoFactory;
import me.danwi.sqlex.core.annotation.repository.SqlExSchema;
import me.danwi.sqlex.core.exception.SqlExException;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.exception.SqlExMigrationException;
import me.danwi.sqlex.core.jdbc.RawSQLExecutor;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.List;

public class Migrator {
    private final Logger logger = LoggerFactory.getLogger(getClass());
    //factory
    private final DaoFactory daoFactory;
    //迁移任务定义
    private final Migration[] migrations;

    public Migrator(DaoFactory factory) {
        SqlExSchema[] schemas = factory.getRepositoryClass().getAnnotationsByType(SqlExSchema.class);
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

        this.daoFactory = factory;
    }

    /**
     * 迁移到最近版本
     *
     * @return 返回成功迁移的版本号
     */
    public int migrate() {
        return migrate(null);
    }

    /**
     * 迁移到最新版本
     *
     * @param callback 迁移回调
     * @return 返回成功迁移的版本号
     */
    public int migrate(MigrateCallback callback) {
        return migrate(migrations.length - 1, callback);
    }

    /**
     * 迁移到指定版本
     *
     * @param version 版本号
     * @return 返回成功迁移的版本号
     */
    public int migrate(int version) {
        return migrate(version, null);
    }

    /**
     * 迁移到指定版本
     *
     * @param version  版本号
     * @param callback 迁移回调
     * @return 返回成功迁移的版本号
     */
    public int migrate(int version, MigrateCallback callback) {
        /*
            流程说明
            迁移过程中会保持一个连接用于持有锁(锁版本表) + 版本信息的修改
            另外每个版本的迁移在新的连接中进行
        */
        //根包
        String rootPackage = daoFactory.getRepositoryClass().getPackage().getName();
        logger.info("准备将数据库({})迁移到 {} 版本", rootPackage, version);
        //保证版本表的存在
        try {
            Connection connection = daoFactory.newConnection();
            RawSQLExecutor executor = daoFactory.getRawSQLExecutor(connection);
            executor.execute("create table if not exists _sqlex_version_(package text not null, version int not null, can_migrate bool not null)");
        } catch (Exception ex) {
            throw new SqlExMigrationException(ex);
        }

        //用于更新迁移信息的连接,手动事务管理
        Connection lockConnection = daoFactory.newConnection();
        //用于更新迁移信息的执行器
        RawSQLExecutor executor = daoFactory.getRawSQLExecutor(lockConnection);
        //连接原本的自动提交属性
        boolean originAutoCommit = false;

        try {
            //设置连接属性
            if (lockConnection.getAutoCommit()) {
                lockConnection.setAutoCommit(false);
                originAutoCommit = true;
            }
            //锁定表,如果表锁定发生异常则终止整个迁移过程
            executor.execute("lock tables _sqlex_version_ write");
            logger.info("获取到全局锁,准备开始迁移");

            //获取当前的版本信息
            VersionInfo versionInfo = null;
            List<VersionInfo> results = executor.query(VersionInfo.class, "select * from _sqlex_version_ where package=?", rootPackage);
            if (!results.isEmpty()) {
                //存在版本信息
                versionInfo = results.get(0);
            }
            //不存在版本信息,插入版本信息
            if (versionInfo == null) {
                executor.execute("insert into _sqlex_version_ values(?, -1, true)", rootPackage);
                versionInfo = new VersionInfo();
                versionInfo.setRootPackage(rootPackage);
                versionInfo.setVersion(-1);
                versionInfo.setCanMigrate(true);
            }
            //如果无法迁移
            if (!versionInfo.getCanMigrate())
                throw new SqlExMigrationException("当前状态为无法执行迁移,可能是上次的迁移没有成功完成,需要人工介入");
            //开始迁移
            //版本号相等,无需迁移
            if (version == versionInfo.getVersion()) {
                logger.info("数据库当前版本已经是 {},无需迁移", versionInfo.getVersion());
                return versionInfo.getVersion();
            }

            logger.info("当前数据库版本 {}, 版本差异 {}", versionInfo.getVersion(), version - versionInfo.getVersion());
            //检查版本范围
            if (version <= versionInfo.getVersion() || version >= migrations.length)
                throw new SqlExMigrationException("错误的版本号,当前版本范围" + versionInfo.getVersion() + "<version<=" + (migrations.length - 1));
            //修改迁移状态
            executor.execute("update _sqlex_version_ set can_migrate=false where package=?", rootPackage);
            //挨个版本迁移
            for (int currentVersion = versionInfo.getVersion() + 1; currentVersion <= version; currentVersion++) {
                logger.info("+ 正在执行 {} 版本的迁移任务", currentVersion);
                //执行回调任务
                if (callback != null) {
                    //回调任务在独立的连接中进行
                    Connection migrateConnection = daoFactory.newConnection();
                    boolean connectionAutoCommit = false;
                    try {
                        //设置连接属性
                        if (migrateConnection.getAutoCommit()) {
                            migrateConnection.setAutoCommit(false);
                            connectionAutoCommit = true;
                        }
                        //执行回调
                        callback.before(currentVersion, daoFactory.getRawSQLExecutor(migrateConnection));
                        //提交
                        migrateConnection.commit();
                    } catch (Exception e) {
                        migrateConnection.rollback();
                        throw e;
                    } finally {
                        //还原原本的自动提交属性
                        if (connectionAutoCommit)
                            migrateConnection.setAutoCommit(false);
                        migrateConnection.close();
                    }
                }
                //执行迁移任务
                Migration migration = migrations[currentVersion];
                String[] sqls = migration.getScripts();
                for (String sql : sqls) {
                    doMigrate(currentVersion, sql);
                }
                //一个版本迁移完成,则更新一下版本号
                executor.execute("update _sqlex_version_ set version=? where package=?", currentVersion, rootPackage);
                //执行回调任务
                if (callback != null) {
                    //回调任务在独立的连接中进行
                    Connection migrateConnection = daoFactory.newConnection();
                    boolean connectionAutoCommit = false;
                    try {
                        //设置连接属性
                        if (migrateConnection.getAutoCommit()) {
                            migrateConnection.setAutoCommit(false);
                            connectionAutoCommit = true;
                        }
                        //执行回调
                        callback.after(currentVersion, daoFactory.getRawSQLExecutor(migrateConnection));
                        //提交
                        migrateConnection.commit();
                    } catch (Exception e) {
                        migrateConnection.rollback();
                        throw e;
                    } finally {
                        //还原原本的自动提交属性
                        if (connectionAutoCommit)
                            migrateConnection.setAutoCommit(false);
                        migrateConnection.close();
                    }
                }
                logger.info("+ {} 版本迁移成功", currentVersion);
            }
            //迁移完成,修改迁移状态
            executor.execute("update _sqlex_version_ set can_migrate=true where package=?", rootPackage);
            //返回版本号
            return version;
        } catch (Exception ex) {
            //迁移过程中出现异常,记录状态信息
            executor.execute("update _sqlex_version_ set can_migrate=false where package=?", rootPackage);
            //包装异常
            if (ex instanceof SqlExMigrationException)
                throw (SqlExMigrationException) ex;
            else
                throw new SqlExMigrationException(ex);
        } finally {
            try {
                try {
                    //提交
                    lockConnection.commit();
                } finally {
                    //解锁表
                    //只要上面锁定代码执行过,该语句就一定会执行
                    //如果该语句执行错误,可能是connection closed
                    //那样session持有的锁也就自动释放了
                    executor.execute("unlock tables");
                    logger.info("数据库({})版本迁移完成,释放全局锁", rootPackage);
                }
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
        try (Connection connection = daoFactory.newConnection()) {
            logger.info("| \t{}", sql);
            //执行语句
            try (Statement statement = connection.createStatement()) {
                statement.execute(sql);
            }
        } catch (SqlExException e) {
            throw new SqlExMigrationException(version, e.getCause());
        } catch (SQLException e) {
            throw new SqlExMigrationException(version, e);
        }
    }
}
