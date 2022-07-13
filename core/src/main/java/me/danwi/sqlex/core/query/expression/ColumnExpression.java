package me.danwi.sqlex.core.query.expression;

public class ColumnExpression implements Expression {
    private final String tableName;
    private final String columnName;

    public ColumnExpression(String tableName, String columnName) {
        this.tableName = tableName;
        this.columnName = columnName;
    }

    @Override
    public String toSQL() {
        return String.format("%s.%s", tableName, columnName);
    }
}
