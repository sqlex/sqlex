package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.util.HashMap;
import java.util.Map;

public class TableUpdate<T> extends WhereBuilder<T> {
    private final TransactionManager transactionManager;
    private final ParameterSetter parameterSetter;
    private final ExceptionTranslator translator;
    //设置的值
    protected Map<String, Object> values = new HashMap<>();

    public TableUpdate(TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        this.transactionManager = transactionManager;
        this.parameterSetter = parameterSetter;
        this.translator = translator;
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
