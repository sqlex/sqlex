package me.danwi.sqlex.core.checker;

import com.mysql.cj.MysqlType;
import me.danwi.sqlex.core.DaoFactory;
import me.danwi.sqlex.core.annotation.SqlExTableInfo;
import me.danwi.sqlex.core.exception.SqlExCheckException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.sql.*;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class Checker {
    private final Logger logger = LoggerFactory.getLogger(getClass());
    //factory
    DaoFactory factory;

    public Checker(DaoFactory factory) {
        this.factory = factory;
    }

    public void check() {
        logger.info("获取({})表结构", factory.getRepositoryClass().getPackage().getName());
        List<TableInfo> repositoryTables = getRepositoryTables();
        logger.info("获取数据库表结构");
        List<TableInfo> databaseTables = getDatabaseTables();
        logger.info("开始比对");
        List<TableInfo> diffTables = diff(repositoryTables, databaseTables);
        if (diffTables.size() != 0) {
            throw new SqlExCheckException(diffTables);
        }
        logger.info("比对完成");
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
                    if (targetColumn == null || sourceColumn.typeId != targetColumn.typeId
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
            ResultSet tableResultSet = databaseMetaData.getTables(conn.getCatalog(), null, null, null);
            List<TableInfo> tables = new ArrayList<>();
            while (tableResultSet.next()) {
                //表名
                String tableName = tableResultSet.getString("TABLE_NAME");
                //列信息
                ResultSet columnResultSet = databaseMetaData.getColumns(conn.getCatalog(), null, tableName, null);
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
                tables.add(new TableInfo(tableName, columns));
            }
            return tables;
        } catch (SQLException e) {
            throw new SqlExCheckException(e);
        }
    }

    private List<TableInfo> getRepositoryTables() {
        List<TableInfo> tables = new ArrayList<>();
        for (SqlExTableInfo t : this.factory.getRepositoryClass().getAnnotationsByType(SqlExTableInfo.class)) {
            List<ColumnInfo> columns = new ArrayList<>();
            for (int i = 0; i < t.columnNames().length; i++) {
                columns.add(new ColumnInfo(
                        t.columnNames()[i],
                        JDBCType.valueOf(t.columnTypeIds()[i]),
                        t.columnTypeNames()[i],
                        t.columnLengths()[i],
                        t.columnUnsigneds()[i]
                ));
            }
            tables.add(new TableInfo(t.name(), columns));
        }
        return tables;
    }
}