package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.annotation.entity.SqlExColumnName;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

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
     * 新建行(插入),返回插入后的实体对象(包含自动填充字段的值)
     *
     * @param entity 需要插入的实体
     * @return 插入后的实体
     */
    public T save(T entity) {
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
                        columnNames.add(columnNameAnnotation.value());
                        parameters.add(readMethod.invoke(entity));
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
            try (PreparedStatement statement = connection.prepareStatement(sql)) {
                parameterSetter.setParameters(statement, parameters);
                //执行
                statement.executeUpdate();
                //TODO: 返回带主键生成的实体
                return entity;
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
}
