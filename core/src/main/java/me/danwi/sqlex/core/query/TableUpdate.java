package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;
import me.danwi.sqlex.core.query.expression.Expression;
import me.danwi.sqlex.core.query.expression.ExpressionUtil;

import java.util.HashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;

public class TableUpdate<T> extends WhereBuilder<T> {
    private final String tableName;
    private final RawSQLExecutor executor;
    //设置的值
    protected Map<String, Object> values = new HashMap<>();

    public TableUpdate(String tableName, RawSQLExecutor executor) {
        this.tableName = tableName;
        this.executor = executor;
    }

    private SQLParameterBind buildSQL() {
        String sql = "update `" + tableName + "` set ";
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
                setSegments.add(String.format("`%s` = %s", columnName, sqlParameterBind.getSQL()));
            } else {
                //添加参数
                parameters.add(value);
                //添加set片段
                setSegments.add(String.format("`%s` = ?", columnName));
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
        //如果没有值,则直接返回0
        if (values.size() == 0)
            return 0;
        //构建SQL
        SQLParameterBind sqlParameterBind = this.buildSQL();
        return executor.execute(null, sqlParameterBind.getSQL(), sqlParameterBind.getParameters()).getAffectRows();
    }
}
