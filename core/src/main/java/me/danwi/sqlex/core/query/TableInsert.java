package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.annotation.entity.SqlExColumnName;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;
import org.jetbrains.annotations.Nullable;

import java.beans.IntrospectionException;
import java.beans.Introspector;
import java.beans.PropertyDescriptor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.sql.*;
import java.util.LinkedList;
import java.util.List;
import java.util.stream.Collectors;

public class TableInsert<T> {
    private final String tableName;
    private final TransactionManager transactionManager;
    private final ParameterSetter parameterSetter;
    private final ExceptionTranslator translator;

    public TableInsert(String tableName, TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        this.tableName = tableName;
        this.transactionManager = transactionManager;
        this.parameterSetter = parameterSetter;
        this.translator = translator;
    }

    /**
     * 新建行(插入)
     *
     * @param entity  需要插入的实体
     * @param options 选项
     * @return 自动生成的主键(没有则为null)
     */
    @Nullable
    public Long insert(T entity, int options) {
        //列名
        List<String> columnNames = new LinkedList<>();
        //参数
        List<Object> parameters = new LinkedList<>();
        try {
            //获取bean的属性
            PropertyDescriptor[] propertyDescriptors = Introspector.getBeanInfo(entity.getClass()).getPropertyDescriptors();
            for (PropertyDescriptor property : propertyDescriptors) {
                //获取它的getter方法
                Method readMethod = property.getReadMethod();
                if (readMethod != null) {
                    //判断其是否为数据列
                    SqlExColumnName columnNameAnnotation = readMethod.getAnnotation(SqlExColumnName.class);
                    if (columnNameAnnotation != null) {
                        Object value = readMethod.invoke(entity);
                        //如果是null,且设置了忽略null值的插入,则忽略该列
                        if ((options & InsertOption.NULL_IS_NONE) == InsertOption.NULL_IS_NONE && value == null)
                            continue;
                        columnNames.add(columnNameAnnotation.value());
                        parameters.add(value);
                    }
                }
            }
        } catch (IntrospectionException | InvocationTargetException | IllegalAccessException e) {
            throw new SqlExImpossibleException("无法获取实体的属性/列信息", e);
        }

        //构建SQL
        String sql = String.format("insert into %s(%s) values(%s)",
                tableName,
                String.join(", ", columnNames),
                columnNames.stream().map(it -> "?").collect(Collectors.joining(", "))
        );
        //如果key重复,则更新
        if ((options & InsertOption.INSERT_OR_UPDATE) == InsertOption.INSERT_OR_UPDATE) {
            sql = sql
                    + " on duplicate key update "
                    + columnNames.stream()
                    .map(it -> String.format("%s = values(%s)", it, it))
                    .collect(Collectors.joining(","));
        }

        //开始准备执行
        //获取事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        Connection connection;
        if (currentTransaction != null)
            //存在事务,则从事务中获取连接
            connection = currentTransaction.getConnection();
        else
            //不存在事务,则新建一个连接
            connection = transactionManager.newConnection();

        //调用方法
        try {
            try (PreparedStatement statement = connection.prepareStatement(sql, Statement.RETURN_GENERATED_KEYS)) {
                parameterSetter.setParameters(statement, parameters);
                //执行
                statement.executeUpdate();
                //获取生成主键信息
                try (ResultSet rs = statement.getGeneratedKeys()) {
                    if (rs.next()) {
                        if (rs.getMetaData().getColumnCount() == 1) {
                            return rs.getLong(1);
                        }
                    }
                }
                //没有主键信息,则返回null
                return null;
            }
        } catch (SQLException e) {
            throw translator.translate(e);
        } finally {
            //不是事务中的连接主要手动关闭
            if (currentTransaction == null) {
                try {
                    connection.close();
                } catch (SQLException e) {
                    //noinspection ThrowFromFinallyBlock
                    throw translator.translate(e);
                }
            }
        }
    }

    /**
     * 新建行(插入)
     *
     * @param entity 需要插入的实体
     * @return 自动生成的主键(没有则为null)
     */
    @Nullable
    public Long insert(T entity) {
        return insert(entity, InsertOption.NONE_OPTIONS);
    }

    /**
     * 新建行(插入),忽略为null的属性
     *
     * @param entity 需要插入的实体
     * @return 自动生成的主键(没有则为null)
     */
    @Nullable
    public Long insertWithoutNull(T entity) {
        return insert(entity, InsertOption.NULL_IS_NONE);
    }

    /**
     * 新建行(插入),如果key冲突则更新
     *
     * @param entity 实体
     * @return 自动生成的主键(没有则为null)
     */
    @Nullable
    public Long upsert(T entity) {
        return insert(entity, InsertOption.INSERT_OR_UPDATE);
    }

    /**
     * 新建行(插入),忽略为null的属性,如果key冲突则更新
     *
     * @param entity 实体
     * @return 自动生成的主键(没有则为null)
     */
    @Nullable
    public Long upsertWithoutNull(T entity) {
        return insert(entity, InsertOption.NULL_IS_NONE | InsertOption.INSERT_OR_UPDATE);
    }
}
