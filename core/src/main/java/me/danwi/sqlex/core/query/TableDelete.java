package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;
import me.danwi.sqlex.core.query.expression.ExpressionUtil;

import java.util.LinkedList;
import java.util.List;

public class TableDelete extends WhereBuilder<TableDelete> {
    private final String tableName;
    private final RawSQLExecutor executor;

    public TableDelete(String tableName, RawSQLExecutor executor) {
        this.tableName = tableName;
        this.executor = executor;
    }

    private SQLParameterBind buildSQL() {
        String sql = "delete from `" + tableName + "`";
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
        //构建SQL
        SQLParameterBind sqlParameterBind = this.buildSQL();
        return executor.execute(null, sqlParameterBind.getSQL(), sqlParameterBind.getParameters()).getAffectRows();
    }
}
