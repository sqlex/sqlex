package me.danwi.sqlex.core.checker;

import java.util.Objects;

public class ColumnInfo {
    String name;
    int typeId;
    String typeName;
    long length;
    boolean unsigned;

    public ColumnInfo(String name, int typeId, String typeName, long length, boolean unsigned) {
        this.name = name;
        this.typeId = typeId;
        this.typeName = typeName;
        this.length = length;
        this.unsigned = unsigned;
    }

    @Override
    public String toString() {
        return "ColumnInfo{" +
                "name='" + name + '\'' +
                ", typeId=" + typeId +
                ", typeName='" + typeName + '\'' +
                ", length=" + length +
                ", unsigned=" + unsigned +
                '}';
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        ColumnInfo that = (ColumnInfo) o;
        return typeId == that.typeId && length == that.length && unsigned == that.unsigned && Objects.equals(name, that.name) && Objects.equals(typeName, that.typeName);
    }

    @Override
    public int hashCode() {
        return Objects.hash(name, typeId, typeName, length, unsigned);
    }
}
