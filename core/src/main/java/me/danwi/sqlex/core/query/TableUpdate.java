package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.query.expression.Expression;
import me.danwi.sqlex.core.query.expression.ExpressionUtil;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.util.HashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;

public class TableUpdate<T> extends WhereBuilder<T> {
    private final String tableName;
    private final TransactionManager transactionManager;
    private final ParameterSetter parameterSetter;
    private final ExceptionTranslator translator;
    //设置的值
    protected Map<String, Object> values = new HashMap<>();

    public TableUpdate(String tableName, TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        this.tableName = tableName;
        this.transactionManager = transactionManager;
        this.parameterSetter = parameterSetter;
        this.translator = translator;
    }

    private SQLParameterBind buildSQL() {
        String sql = "update `" + tableName + "`";
        List<Object> parameters = new LinkedList<>();
        //添加set部分
        List<String> setSegments = new LinkedList<>();
        for (Map.Entry<String, Object> entry : values.entrySet()) {
            String columnName = entry.getKey();
            Object value = entry.getValue();
            if (value instanceof Expression) {
                //如果是表达式,则需要表达式求值
                SQLParameterBind sqlParameterBind = ExpressionUtil.toSQL((Expression) value);
                ///添加参数
                parameters.addAll(sqlParameterBind.getParameters());
                //添加set片段
                setSegments.add(String.format("set `%s` = %s", columnName, sqlParameterBind.getSQL()));
            } else {
                //添加参数
                parameters.add(value);
                //添加set片段
                setSegments.add(String.format("set `%s` = ?", columnName));
            }
        }
        sql = sql + " " + String.join(", ", setSegments);
        //处理where条件
        if (this.whereCondition != null) {
            SQLParameterBind sqlParameterBind = ExpressionUtil.toSQL(this.whereCondition);
            sql = sql + " where " + sqlParameterBind.getSQL();
            parameters.addAll(sqlParameterBind.getParameters());
        }
        return new SQLParameterBind(sql, parameters);
    }

    /**
     * 执行更新
     *
     * @return 更新的行数
     */
    public long execute() {
        //获取事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        Connection connection;
        if (currentTransaction != null)
            //存在事务,则从事务中获取连接
            connection = currentTransaction.getConnection();
        else
            //不存在事务,则新建一个连接
            connection = transactionManager.newConnection();

        //调用方法
        try {
            //构建SQL
            SQLParameterBind sqlParameterBind = this.buildSQL();
            //新建预处理数据
            try (PreparedStatement statement = connection.prepareStatement(sqlParameterBind.getSQL())) {
                parameterSetter.setParameters(statement, sqlParameterBind.getParameters());
                try {
                    return statement.executeLargeUpdate();
                } catch (UnsupportedOperationException e) {
                    return statement.executeUpdate();
                }
            }
        } catch (SQLException e) {
            throw translator.translate(e);
        } finally {
            //不是事务中的连接主要手动关闭
            if (currentTransaction == null) {
                try {
                    connection.close();
                } catch (SQLException e) {
                    //noinspection ThrowFromFinallyBlock
                    throw translator.translate(e);
                }
            }
        }
    }
}
