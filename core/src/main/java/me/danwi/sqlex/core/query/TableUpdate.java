package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.transaction.TransactionManager;

import java.util.HashMap;
import java.util.Map;

public class TableUpdate<T> extends WhereBuilder<T> {
    private final TransactionManager transactionManager;
    //设置的值
    protected Map<String, Object> values = new HashMap<>();

    public TableUpdate(TransactionManager transactionManager) {
        this.transactionManager = transactionManager;
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
