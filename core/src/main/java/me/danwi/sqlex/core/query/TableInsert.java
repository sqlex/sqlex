package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.annotation.entity.SqlExColumnName;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.jdbc.RawSQLExecutor;
import org.jetbrains.annotations.Nullable;

import java.beans.IntrospectionException;
import java.beans.Introspector;
import java.beans.PropertyDescriptor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.LinkedList;
import java.util.List;
import java.util.stream.Collectors;

/**
 * 表插入
 *
 * @param <T> 实体类型
 * @param <K> 自动生成字段的类型
 */
public class TableInsert<T, K> {
    private final String tableName;
    private final RawSQLExecutor executor;
    private final Class<K> generatedColumnJavaType;


    public TableInsert(String tableName, RawSQLExecutor executor, Class<K> generatedColumnJavaType) {
        this.tableName = tableName;
        this.executor = executor;
        this.generatedColumnJavaType = generatedColumnJavaType;
    }

    /**
     * 新建行(插入)
     *
     * @param entity  需要插入的实体
     * @param options 选项
     * @return 自动生成的列的值(没有则为null)
     */
    @Nullable
    public K insert(T entity, int options) {
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
        String sql = String.format("insert into `%s`(%s) values(%s)",
                tableName,
                columnNames.stream().map(name -> "`" + name + "`").collect(Collectors.joining(", ")),
                columnNames.stream().map(it -> "?").collect(Collectors.joining(", "))
        );
        //如果key重复,则更新
        if ((options & InsertOption.INSERT_OR_UPDATE) == InsertOption.INSERT_OR_UPDATE) {
            sql = sql
                    + " on duplicate key update "
                    + columnNames.stream()
                    .map(it -> String.format("`%s` = values(`%s`)", it, it))
                    .collect(Collectors.joining(","));
        }

        //执行
        return executor.execute(this.generatedColumnJavaType, sql, parameters).getGeneratedKey();
    }

    /**
     * 新建行(插入)
     *
     * @param entity 需要插入的实体
     * @return 自动生成的列的值(没有则为null)
     */
    @Nullable
    public K insert(T entity) {
        return insert(entity, InsertOption.NONE_OPTIONS);
    }

    /**
     * 新建行(插入),忽略为null的属性
     *
     * @param entity 需要插入的实体
     * @return 自动生成的列的值(没有则为null)
     */
    @Nullable
    public K insertWithoutNull(T entity) {
        return insert(entity, InsertOption.NULL_IS_NONE);
    }

    /**
     * 新建行(插入),如果key冲突则更新
     *
     * @param entity 实体
     * @return 自动生成的列的值(没有则为null)
     */
    @Nullable
    public K upsert(T entity) {
        return insert(entity, InsertOption.INSERT_OR_UPDATE);
    }

    /**
     * 新建行(插入),忽略为null的属性,如果key冲突则更新
     *
     * @param entity 实体
     * @return 自动生成的列的值(没有则为null)
     */
    @Nullable
    public K upsertWithoutNull(T entity) {
        return insert(entity, InsertOption.NULL_IS_NONE | InsertOption.INSERT_OR_UPDATE);
    }
}
