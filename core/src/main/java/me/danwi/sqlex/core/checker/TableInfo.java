package me.danwi.sqlex.core.checker;

import java.util.List;

public class TableInfo {
    String name;

    List<ColumnInfo> columns;

    public TableInfo(String name, List<ColumnInfo> columns) {
        this.name = name;
        this.columns = columns;
    }

    public String getName() {
        return name;
    }

    public List<ColumnInfo> getColumns() {
        return columns;
    }

    @Override
    public String toString() {
        StringBuilder builder = new StringBuilder();
        builder.append(name).append(" {\n");
        for (ColumnInfo column : columns) {
            builder.append("\t").append(column.toString()).append("\n");
        }
        builder.append("}");
        return builder.toString();
    }
}
