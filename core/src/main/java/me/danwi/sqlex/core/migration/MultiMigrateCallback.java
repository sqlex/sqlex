package me.danwi.sqlex.core.migration;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

import java.util.LinkedList;
import java.util.List;

/**
 * 多个迁移回调组合
 */
public class MultiMigrateCallback implements MigrateCallback {
    private final List<MigrateCallback> callbacks;

    public MultiMigrateCallback() {
        this.callbacks = new LinkedList<>();
    }

    public MultiMigrateCallback(List<MigrateCallback> callbacks) {
        this.callbacks = callbacks;
    }

    public void add(MigrateCallback callback) {
        this.callbacks.add(callback);
    }

    public void remove(MigrateCallback callback) {
        this.callbacks.remove(callback);
    }

    public void clear() {
        this.callbacks.clear();
    }

    @Override
    public void before(int version, RawSQLExecutor executor) throws Exception {
        for (MigrateCallback callback : callbacks)
            callback.before(version, executor);
    }

    @Override
    public void after(int version, RawSQLExecutor executor) throws Exception {
        for (MigrateCallback callback : callbacks)
            callback.after(version, executor);
    }
}
