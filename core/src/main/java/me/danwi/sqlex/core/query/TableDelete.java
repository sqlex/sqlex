package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.transaction.TransactionManager;

public class TableDelete extends WhereBuilder<TableDelete> {
    private final TransactionManager transactionManager;
    private final ParameterSetter parameterSetter;
    private final ExceptionTranslator translator;

    public TableDelete(TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        this.transactionManager = transactionManager;
        this.parameterSetter = parameterSetter;
        this.translator = translator;
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
