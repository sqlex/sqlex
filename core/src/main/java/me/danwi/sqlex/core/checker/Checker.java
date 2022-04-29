package me.danwi.sqlex.core.checker;

import com.mysql.cj.MysqlType;
import me.danwi.sqlex.core.DaoFactory;
import me.danwi.sqlex.core.annotation.SqlExTableInfo;
import me.danwi.sqlex.core.exception.SqlExCheckException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.sql.*;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

public class Checker {
    private final Logger logger = LoggerFactory.getLogger(getClass());
    //factory
    DaoFactory factory;

    public Checker(DaoFactory factory) {
        this.factory = factory;
    }

    public void check() {
        logger.info("获取repo表结构");
        List<TableInfo> annotationTables = new ArrayList<>();
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
            annotationTables.add(new TableInfo(t.name(), columns));
        }
        logger.info("获取数据库表结构");
        List<TableInfo> databaseTables = getDatabaseTables();
        logger.info("开始比对");
        List<TableInfo> diffTables = diff(annotationTables, databaseTables);
        if (diffTables.size() != 0) {
            throw new SqlExCheckException(diffTables);
        }
        logger.info("比对完成");
    }

    /**
     * 表信息比对
     *
     * @param sourceTables 源
     * @param targetTables 目标
     * @return 源 - 目标
     */
    private List<TableInfo> diff(List<TableInfo> sourceTables, List<TableInfo> targetTables) {
        List<TableInfo> diffTables = new ArrayList<>();
        sourceTable:
        for (TableInfo source : sourceTables) {
            for (TableInfo target : targetTables) {
                if (Objects.equals(source.name, target.name)) {
                    List<ColumnInfo> diffColumns = new ArrayList<>();
                    for (ColumnInfo sc : source.columns) {
                        for (ColumnInfo tc : target.columns) {
                            if (Objects.equals(sc.name, tc.name)) {
                                //repository的tinyint(1) 等同于 数据库的bit(1)
                                if (sc.typeId == JDBCType.TINYINT && sc.length == 1 && tc.typeId == JDBCType.BIT && tc.length == 1)
                                    continue;
                                if (!(sc.typeId == tc.typeId) || MysqlType.isSigned(MysqlType.getByJdbcType(sc.typeId.getVendorTypeNumber())) && sc.unsigned != tc.unsigned) {
                                    diffColumns.add(sc);
                                }
                            }
                        }
                    }
                    if (diffColumns.size() > 0) {
                        diffTables.add(new TableInfo(source.name, diffColumns));
                    }
                    continue sourceTable;
                }
            }
            diffTables.add(source);
        }

        return diffTables;
    }

    /**
     * 获取数据库表信息
     *
     * @return 表信息数组
     */
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
}