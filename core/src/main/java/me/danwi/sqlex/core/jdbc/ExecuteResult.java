package me.danwi.sqlex.core.jdbc;

/**
 * SQL语句执行结果
 *
 * @param <K> 生成键对应的Java类型
 */
public class ExecuteResult<K> {
    private final long affectRows;
    private final K generatedKey;

    ExecuteResult(long affectRows, K generatedKey) {
        this.affectRows = affectRows;
        this.generatedKey = generatedKey;
    }

    /**
     * @return 返回影响的行数
     */
    public long getAffectRows() {
        return affectRows;
    }

    /**
     * @return 生成键的值, 如果没有则为空
     */
    public K getGeneratedKey() {
        return generatedKey;
    }
}
