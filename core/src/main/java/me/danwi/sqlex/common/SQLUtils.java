package me.danwi.sqlex.common;


import com.alibaba.druid.sql.ast.SQLExpr;
import com.alibaba.druid.sql.ast.SQLStatement;
import com.alibaba.druid.sql.ast.expr.SQLIdentifierExpr;
import com.alibaba.druid.sql.ast.expr.SQLPropertyExpr;
import com.alibaba.druid.sql.ast.statement.SQLExprTableSource;
import com.alibaba.druid.sql.dialect.mysql.visitor.MySqlASTVisitor;

import java.util.LinkedList;
import java.util.List;
import java.util.Map;

/**
 * SQL辅助工具
 */
public class SQLUtils {
    /**
     * 根据mapping将SQL中的数据库名做重映射
     *
     * @param sql     SQL语句
     * @param mapping 数据库名映射
     * @return 返回修改了数据库名后的SQL
     */
    public static String replaceDatabaseName(String sql, Map<String, String> mapping) {
        final boolean[] isReplaced = {false};
        SQLStatement sqlStatement = com.alibaba.druid.sql.SQLUtils.parseSingleMysqlStatement(sql);
        final List<String> databaseNames = new LinkedList<>();
        //替换表引用
        sqlStatement.accept(new MySqlASTVisitor() {
            @Override
            public boolean visit(SQLExprTableSource table) {
                String databaseName = table.getSchema();
                if (databaseName != null) {
                    String actualDatabaseName = mapping.get(databaseName);
                    if (actualDatabaseName != null) {
                        table.setSchema(actualDatabaseName);
                        isReplaced[0] = true;
                    }
                    databaseNames.add(databaseName);
                }
                return false;
            }
        });
        //替换表达式中的表引用
        sqlStatement.accept(new MySqlASTVisitor() {
            @Override
            public boolean visit(SQLPropertyExpr x) {
                //TODO: 目前的分析和替换是不严格的可能会出现错误
                SQLExpr tableExpr = x.getOwner();
                if (tableExpr != null) {
                    if (tableExpr instanceof SQLPropertyExpr) {
                        SQLExpr databaseExpr = ((SQLPropertyExpr) tableExpr).getOwner();
                        if (databaseExpr != null) {
                            if (databaseExpr instanceof SQLIdentifierExpr) {
                                String databaseName = ((SQLIdentifierExpr) databaseExpr).getName();
                                if (databaseNames.stream().anyMatch(it -> it.equals(databaseName))) {
                                    //在上下文中存在对应的数据库名
                                    String actualDatabaseName = mapping.get(databaseName);
                                    if (actualDatabaseName != null) {
                                        ((SQLIdentifierExpr) databaseExpr).setName(actualDatabaseName);
                                        isReplaced[0] = true;
                                    }
                                }
                            }
                        }
                    }
                }
                return false;
            }
        });
        if (!isReplaced[0])
            return sql;
        return sqlStatement.toString();
    }
}
