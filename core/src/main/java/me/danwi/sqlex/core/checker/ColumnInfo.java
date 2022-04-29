package me.danwi.sqlex.core.checker;

import java.sql.JDBCType;
import java.util.Objects;

public class ColumnInfo {
    String name;
    JDBCType typeId;
    String typeName;
    long length;
    boolean unsigned;

    public ColumnInfo(String name, JDBCType typeId, String typeName, long length, boolean unsigned) {
        this.name = name;
        this.typeId = typeId;
        this.typeName = typeName;
        this.length = length;
        this.unsigned = unsigned;
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

    @Override
    public String toString() {
        return name + " " + typeName;
    }
}
