package me.danwi.sqlex.core.checker;

import com.mysql.cj.MysqlType;
import me.danwi.sqlex.core.DaoFactory;
import me.danwi.sqlex.core.annotation.SqlExTableColumn;
import me.danwi.sqlex.core.exception.SqlExCheckException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLException;
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
        List<TableColumn> annotationTables = new ArrayList<>();
        for (SqlExTableColumn t : this.factory.getRepositoryClass().getAnnotationsByType(SqlExTableColumn.class)) {
            annotationTables.add(new TableColumn(t.tableName(), t.columnName(), t.columnType(), Long.parseLong(t.columnLength()), Boolean.parseBoolean(t.columnUnsigned())));
        }
        logger.info("获取数据库表结构");
        List<TableColumn> databaseTables;
        try {
            databaseTables = getDatabaseTables();
        } catch (Exception e) {
            throw new SqlExCheckException("获取数据库表信息失败", e);
        }
        logger.info("开始比对");
        List<TableColumn> diffTables = diff(annotationTables, databaseTables);
        if (diffTables.size() != 0) {
            throw new SqlExCheckException();
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
    private List<TableColumn> diff(List<TableColumn> sourceTables, List<TableColumn> targetTables) {
        List<TableColumn> diffTables = new ArrayList<>();
        for (TableColumn source : sourceTables) {
            boolean isDiff = true;
            for (TableColumn target : targetTables) {
                //宽松模式比对
                boolean loose = Objects.equals(source.tableName, target.tableName) && Objects.equals(source.columnName, target.columnName) && Objects.equals(source.columnType, target.columnType);
                if (loose) {
                    isDiff = false;
                    break;
                }
            }
            if (isDiff) {
                logger.warn("定义的表 {} 字段 {} 与数据库不一致", source.tableName, source.columnName);
                diffTables.add(source);
            }
        }

        return diffTables;
    }

    /**
     * 获取数据库表信息
     *
     * @return 表信息数组
     */
    private List<TableColumn> getDatabaseTables() throws SQLException {
        Connection conn = this.factory.newConnection();
        DatabaseMetaData databaseMetaData = conn.getMetaData();
        //获取所有的表名
        ResultSet tableResultSet = databaseMetaData.getTables(conn.getCatalog(), null, null, null);
        List<TableColumn> tables = new ArrayList<>();
        while (tableResultSet.next()) {
            //表名
            String tableName = tableResultSet.getString("TABLE_NAME");
            //列信息
            ResultSet columnResultSet = databaseMetaData.getColumns(conn.getCatalog(), null, tableName, null);
            while (columnResultSet.next()) {
                tables.add(new TableColumn(tableName, columnResultSet.getString("COLUMN_NAME"), MysqlType.getByJdbcType(columnResultSet.getInt("DATA_TYPE")).getName().toLowerCase(), (long) columnResultSet.getInt("COLUMN_SIZE"), columnResultSet.getString("TYPE_NAME").contains("UNSIGNED")));
            }
        }
        return tables;
    }
}