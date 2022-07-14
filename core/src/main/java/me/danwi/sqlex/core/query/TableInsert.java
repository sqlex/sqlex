package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.transaction.TransactionManager;

public class TableInsert<T> {
    private final TransactionManager transactionManager;
    private final Class<T> entityClass;

    public TableInsert(TransactionManager transactionManager, Class<T> entityClass) {
        this.transactionManager = transactionManager;
        this.entityClass = entityClass;
    }

    /**
     * 新建行(插入),返回插入后的实体对象(包含自动填充字段的值)
     *
     * @param person 需要插入的实体
     * @return 插入后的实体
     */
    public T save(T person) {
        throw new UnsupportedOperationException();
    }
}
