package me.danwi.sqlex.core.checker;

import com.mysql.cj.MysqlType;
import me.danwi.sqlex.core.DaoFactory;
import me.danwi.sqlex.core.annotation.repository.SqlExTables;
import me.danwi.sqlex.core.exception.SqlExCheckException;
import me.danwi.sqlex.core.query.Column;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.sql.*;
import java.util.*;

public class Checker {
    private final Logger logger = LoggerFactory.getLogger(getClass());
    //factory
    DaoFactory factory;

    public Checker(DaoFactory factory) {
        this.factory = factory;
    }

    public void check() {
        logger.info("准备比对数据库结构一致性");
        logger.info("获取SqlEx Repository({})定义结构", factory.getRepositoryClass().getPackage().getName());
        List<TableInfo> repositoryTables = getRepositoryTables();
        logger.info("获取目标数据库结构");
        List<TableInfo> databaseTables = getDatabaseTables();
        logger.info("计算结构差异");
        List<TableInfo> diffTables = diff(repositoryTables, databaseTables);
        if (diffTables.size() != 0) {
            throw new SqlExCheckException(diffTables);
        }
        logger.info("结构一致性比对完成,无差异");
    }

    private List<TableInfo> diff(List<TableInfo> source, List<TableInfo> target) {
        Map<String, Map<String, ColumnInfo>> sourceTables = toTableColumnMap(source);
        Map<String, Map<String, ColumnInfo>> targetTables = toTableColumnMap(target);

        List<TableInfo> diffTables = new ArrayList<>();
        sourceTables.forEach((sourceTableName, sourceColumnMap) -> {
            Map<String, ColumnInfo> targetColumnMap = targetTables.get(sourceTableName);
            if (targetColumnMap != null) { //数据库表存在, 比对列信息
                List<ColumnInfo> diffColumns = new ArrayList<>();
                sourceColumnMap.forEach((sourceColumnName, sourceColumn) -> {
                    ColumnInfo targetColumn = targetColumnMap.get(sourceColumnName);
                    //repository的tinyint(1) 等同于 数据库的bit(1)
                    if (targetColumn != null && sourceColumn.typeId == JDBCType.TINYINT && sourceColumn.length == 1 && targetColumn.typeId == JDBCType.BIT && targetColumn.length == 1) {
                        return;
                    }
                    if (targetColumn == null ||
                            sourceColumn.typeId != targetColumn.typeId || //数据类型不匹配
                            sourceColumn.primaryKey != targetColumn.primaryKey //主键不匹配
                    ) {
                        diffColumns.add(sourceColumn);
                    }
                });
                if (diffColumns.size() > 0) {
                    diffTables.add(new TableInfo(sourceTableName, diffColumns));
                }
            } else {//数据库表不存在
                diffTables.add(new TableInfo(sourceTableName, new ArrayList<>(sourceColumnMap.values())));
            }
        });

        return diffTables;
    }

    private Map<String, Map<String, ColumnInfo>> toTableColumnMap(List<TableInfo> tables) {
        Map<String, Map<String, ColumnInfo>> map = new HashMap<>();
        for (TableInfo table : tables) {
            Map<String, ColumnInfo> columnMap = new HashMap<>();
            for (ColumnInfo column : table.columns) {
                columnMap.put(column.name, column);
            }
            map.put(table.name, columnMap);
        }
        return map;
    }

    private List<TableInfo> getDatabaseTables() {
        try (Connection conn = this.factory.newConnection()) {
            DatabaseMetaData databaseMetaData = conn.getMetaData();
            //获取所有的表名
            try (ResultSet tableResultSet = databaseMetaData.getTables(conn.getCatalog(), null, null, null)) {
                List<TableInfo> tables = new ArrayList<>();
                while (tableResultSet.next()) {
                    //表名
                    String tableName = tableResultSet.getString("TABLE_NAME");
                    //列信息
                    try (ResultSet columnResultSet = databaseMetaData.getColumns(conn.getCatalog(), null, tableName, null)) {
                        List<ColumnInfo> columns = new ArrayList<>();
                        while (columnResultSet.next()) {
                            columns.add(new ColumnInfo(
                                    columnResultSet.getString("COLUMN_NAME"),
                                    JDBCType.valueOf(columnResultSet.getInt("DATA_TYPE")),
                                    MysqlType.getByJdbcType(columnResultSet.getInt("DATA_TYPE")).getName().toLowerCase(),
                                    columnResultSet.getInt("COLUMN_SIZE"),
                                    columnResultSet.getString("TYPE_NAME").contains("UNSIGNED")
                            ));
                        }
                        //主键
                        ResultSet primaryKeyResultSet = databaseMetaData.getPrimaryKeys(null, null, tableName);
                        while (primaryKeyResultSet.next()) {
                            String columnName = primaryKeyResultSet.getString("COLUMN_NAME");
                            for (ColumnInfo c : columns) {
                                if (Objects.equals(c.name, columnName)) {
                                    c.setPrimaryKey(true);
                                }
                            }
                        }
                        tables.add(new TableInfo(tableName, columns));
                    }
                }
                return tables;
            }
        } catch (SQLException e) {
            throw factory.getExceptionTranslator().translate(e);
        }
    }

    private List<TableInfo> getRepositoryTables() {
        List<TableInfo> tables = new ArrayList<>();
        for (Class tableClass : this.factory.getRepositoryClass().getAnnotation(SqlExTables.class).value()) {
            List<ColumnInfo> columns = new ArrayList<>();
            String tableName = "";
            for (Field field : tableClass.getFields()) {
                try {
                    //静态且是Column类
                    Object instance = field.get(null);
                    if (Modifier.isStatic(field.getModifiers()) && instance instanceof Column) {
                        //获取元数据
                        Column.MetaData metaData = ((Column) instance).getMetaData();
                        columns.add(new ColumnInfo(
                                metaData.isPrimaryKey(),
                                metaData.getColumnName(),
                                metaData.getJdbcType(),
                                metaData.getTypeName(),
                                metaData.getLength(),
                                metaData.isUnsigned()
                        ));
                        tableName = metaData.getTableName();
                    }
                } catch (Exception e) {
                    logger.warn("获取column信息失败: {}", e.toString());
                }
            }
            tables.add(new TableInfo(tableName, columns));
        }
        return tables;
    }
}