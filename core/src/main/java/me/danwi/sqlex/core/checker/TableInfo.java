package me.danwi.sqlex.core.checker;

import java.util.List;

public class TableInfo {
    String name;

    List<ColumnInfo> columns;

    public TableInfo(String name, List<ColumnInfo> columns) {
        this.name = name;
        this.columns = columns;
    }

    @Override
    public String toString() {
        return "TableInfo{" +
                "name='" + name + '\'' +
                ", columns=" + columns +
                '}';
    }
}
