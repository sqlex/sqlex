package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.query.expression.ExpressionUtil;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.util.LinkedList;
import java.util.List;

public class TableDelete extends WhereBuilder<TableDelete> {
    private final String tableName;
    private final TransactionManager transactionManager;
    private final ParameterSetter parameterSetter;
    private final ExceptionTranslator translator;

    public TableDelete(String tableName, TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        this.tableName = tableName;
        this.transactionManager = transactionManager;
        this.parameterSetter = parameterSetter;
        this.translator = translator;
    }

    private SQLParameterBind buildSQL() {
        String sql = "delete from " + tableName;
        List<Object> parameters = new LinkedList<>();
        //处理where条件
        if (this.whereCondition != null) {
            SQLParameterBind sqlParameterBind = ExpressionUtil.toSQL(this.whereCondition);
            sql = sql + " where " + sqlParameterBind.getSQL();
            parameters.addAll(sqlParameterBind.getParameters());
        }
        return new SQLParameterBind(sql, parameters);
    }

    /**
     * 执行删除
     *
     * @return 删除的行数
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
