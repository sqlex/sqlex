package me.danwi.sqlex.core.query.expression;

import java.sql.JDBCType;

public class ColumnExpression implements Expression {
    public static class MetaData {
        private final String tableName;
        private final String columnName;
        private final String typeName;
        private final JDBCType jdbcType;
        private final long length;
        private final boolean unsigned;
        private final boolean binary;
        private final long decimal;
        private final boolean isPrimaryKey;
        private final boolean isAutoIncrement;
        private final boolean isUnique;
        private final boolean isNotNull;
        private final boolean hasDefaultValue;

        MetaData(String tableName, String columnName,
                 String typeName, JDBCType jdbcType, long length,
                 boolean unsigned, boolean binary, long decimal,
                 boolean isPrimaryKey, boolean isAutoIncrement, boolean isUnique,
                 boolean isNotNull, boolean hasDefaultValue) {
            this.tableName = tableName;
            this.columnName = columnName;
            this.typeName = typeName;
            this.jdbcType = jdbcType;
            this.length = length;
            this.unsigned = unsigned;
            this.binary = binary;
            this.decimal = decimal;
            this.isPrimaryKey = isPrimaryKey;
            this.isAutoIncrement = isAutoIncrement;
            this.isUnique = isUnique;
            this.isNotNull = isNotNull;
            this.hasDefaultValue = hasDefaultValue;
        }

        public String getTableName() {
            return tableName;
        }

        public String getColumnName() {
            return columnName;
        }

        public String getTypeName() {
            return typeName;
        }

        public JDBCType getJdbcType() {
            return jdbcType;
        }

        public long getLength() {
            return length;
        }

        public boolean isUnsigned() {
            return unsigned;
        }

        public boolean isBinary() {
            return binary;
        }

        public long getDecimal() {
            return decimal;
        }

        public boolean isPrimaryKey() {
            return isPrimaryKey;
        }

        public boolean isAutoIncrement() {
            return isAutoIncrement;
        }

        public boolean isUnique() {
            return isUnique;
        }

        public boolean isNotNull() {
            return isNotNull;
        }

        public boolean isHasDefaultValue() {
            return hasDefaultValue;
        }
    }

    private final MetaData metaData;

    public ColumnExpression(String tableName, String columnName,
                            String typeName, JDBCType jdbcType, long length,
                            boolean unsigned, boolean binary, long decimal,
                            boolean isPrimaryKey, boolean isAutoIncrement, boolean isUnique,
                            boolean isNotNull, boolean hasDefaultValue) {
        metaData = new MetaData(tableName, columnName,
                typeName, jdbcType, length,
                unsigned, binary, decimal,
                isPrimaryKey, isAutoIncrement, isUnique,
                isNotNull, hasDefaultValue);
    }

    @Override
    public String toSQL() {
        return String.format("%s.%s", this.metaData.tableName, this.metaData.columnName);
    }

    public MetaData getMetaData() {
        return metaData;
    }
}
