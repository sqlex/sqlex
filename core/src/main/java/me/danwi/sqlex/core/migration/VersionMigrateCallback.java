package me.danwi.sqlex.core.migration;


import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

/**
 * 版本化迁移回调
 */
public interface VersionMigrateCallback {
    static MigrateCallback before(int version, VersionMigrateCallback callback) {
        return new MigrateCallback() {
            @Override
            public void before(int currentVersion, RawSQLExecutor executor) throws Exception {
                if (currentVersion == version) {
                    callback.run(executor);
                }
            }

            @Override
            public void after(int currentVersion, RawSQLExecutor executor) {
            }
        };
    }

    static MigrateCallback after(int version, VersionMigrateCallback callback) {
        return new MigrateCallback() {
            @Override
            public void before(int currentVersion, RawSQLExecutor executor) {
            }

            @Override
            public void after(int currentVersion, RawSQLExecutor executor) throws Exception {
                if (currentVersion == version) {
                    callback.run(executor);
                }
            }
        };
    }

    void run(RawSQLExecutor executor) throws Exception;
}
