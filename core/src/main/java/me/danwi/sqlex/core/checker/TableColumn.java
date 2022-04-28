package me.danwi.sqlex.core.checker;

import java.util.Objects;

public class TableColumn {
    public String tableName;

    public String columnName;

    public String columnType;

    public Long columnLength;

    public boolean columnUnsigned;

    public TableColumn(String tableName, String columnName, String columnType, Long columnLength, boolean columnUnsigned) {
        this.tableName = tableName;
        this.columnName = columnName;
        this.columnType = columnType;
        this.columnLength = columnLength;
        this.columnUnsigned = columnUnsigned;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        TableColumn that = (TableColumn) o;
        return columnUnsigned == that.columnUnsigned && Objects.equals(tableName, that.tableName) && Objects.equals(columnName, that.columnName) && Objects.equals(columnType, that.columnType) && Objects.equals(columnLength, that.columnLength);
    }

    @Override
    public int hashCode() {
        return Objects.hash(tableName, columnName, columnType, columnLength, columnUnsigned);
    }

    @Override
    public String toString() {
        return "TableColumn{" +
                "tableName='" + tableName + '\'' +
                ", columnName='" + columnName + '\'' +
                ", columnType='" + columnType + '\'' +
                ", columnLength='" + columnLength + '\'' +
                ", columnUnsigned=" + columnUnsigned +
                '}';
    }
}
