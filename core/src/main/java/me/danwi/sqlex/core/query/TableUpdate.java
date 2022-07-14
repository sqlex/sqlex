package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.transaction.TransactionManager;

import java.util.HashMap;
import java.util.Map;

public class TableUpdate<T> extends WhereBuilder<T> {
    private final TransactionManager transactionManager;
    private final Class<?> entityClass;

    //设置的值
    protected Map<String, Object> values = new HashMap<>();

    public TableUpdate(TransactionManager transactionManager, Class<?> entityClass) {
        this.transactionManager = transactionManager;
        this.entityClass = entityClass;
    }

    /**
     * 执行更新
     *
     * @return 更新的行数
     */
    public long execute() {
        throw new UnsupportedOperationException();
    }
}
