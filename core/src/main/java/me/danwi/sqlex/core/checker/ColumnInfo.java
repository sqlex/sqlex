package me.danwi.sqlex.core.checker;

import java.sql.JDBCType;

public class ColumnInfo {
    boolean primaryKey;
    String name;
    JDBCType typeId;
    String typeName;
    long length;
    boolean unsigned;

    public ColumnInfo(boolean primaryKey, String name, JDBCType typeId, String typeName, long length, boolean unsigned) {
        this.primaryKey = primaryKey;
        this.name = name;
        this.typeId = typeId;
        this.typeName = typeName;
        this.length = length;
        this.unsigned = unsigned;
    }

    public ColumnInfo(String name, JDBCType typeId, String typeName, long length, boolean unsigned) {
        this(false, name, typeId, typeName, length, unsigned);
    }

    public String getName() {
        return name;
    }

    public JDBCType getTypeId() {
        return typeId;
    }

    public String getTypeName() {
        return typeName;
    }

    public long getLength() {
        return length;
    }

    public boolean isUnsigned() {
        return unsigned;
    }

    public boolean isPrimaryKey() {
        return primaryKey;
    }

    public void setPrimaryKey(boolean primaryKey) {
        this.primaryKey = primaryKey;
    }

    @Override
    public String toString() {
        return name + " " + typeName;
    }
}
