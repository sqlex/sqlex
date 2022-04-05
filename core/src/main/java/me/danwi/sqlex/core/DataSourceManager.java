package me.danwi.sqlex.core;

public class DataSourceManager {
    public <R extends RepositoryLike> DataSource<R> getInstance(Class<R> repository) {
        return new DataSource<>();
    }
}
