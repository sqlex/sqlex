package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.transaction.TransactionManager;

public class TableDelete extends WhereBuilder<TableDelete> {
    private final TransactionManager transactionManager;

    public TableDelete(TransactionManager transactionManager) {
        this.transactionManager = transactionManager;
    }

    /**
     * 执行删除
     *
     * @return 删除的行数
     */
    public long execute() {
        throw new UnsupportedOperationException();
    }
}
