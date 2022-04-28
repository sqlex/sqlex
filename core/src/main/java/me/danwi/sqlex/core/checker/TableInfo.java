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
}
