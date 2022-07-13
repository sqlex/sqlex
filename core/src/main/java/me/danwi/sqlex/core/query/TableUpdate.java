package me.danwi.sqlex.core.query;

public class TableUpdate<T> extends WhereBuilder<T> {
    /**
     * 执行更新
     *
     * @return 更新的行数
     */
    long execute() {
        throw new UnsupportedOperationException();
    }
}
