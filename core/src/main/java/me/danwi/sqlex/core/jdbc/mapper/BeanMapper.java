package me.danwi.sqlex.core.jdbc.mapper;

import me.danwi.sqlex.core.annotation.entity.SqlExColumnName;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;

import java.beans.IntrospectionException;
import java.beans.Introspector;
import java.beans.PropertyDescriptor;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.LinkedList;
import java.util.List;
import java.util.Objects;

public class BeanMapper<T> extends RowMapper<T> {
    //实体类
    private final Class<T> beanClass;
    //实体类构造函数
    private final Constructor<T> beanConstructor;
    //实体类属性信息缓存
    private PropertyInfo[] beanPropertyInfoCaches;

    public BeanMapper(Class<T> bean) {
        beanClass = bean;
        try {
            beanConstructor = beanClass.getDeclaredConstructor();
        } catch (Exception e) {
            throw new SqlExImpossibleException("无法获取实体类(" + beanClass.getName() + ")的构造函数");
        }
    }

    private static class PropertyInfo {
        public String name; //属性名
        public String columnName; //列名
        public int columnIndex; //属性在result set中对应的索引
        public Method writeMethod;  //写入方法
        public Class<?> dataType; //数据类型

        public PropertyInfo(String name, String columnName, Method writeMethod, Class<?> dataType) {
            this.name = name;
            this.columnName = columnName;
            this.columnIndex = -1;
            this.writeMethod = writeMethod;
            this.dataType = dataType;
        }
    }

    private PropertyInfo[] getPropertyInfo(ResultSet resultSet) throws SQLException {
        PropertyInfo[] propertyInfo = this.beanPropertyInfoCaches;
        if (propertyInfo == null) {
            synchronized (this) {
                propertyInfo = this.beanPropertyInfoCaches;
                if (propertyInfo == null) {
                    //建立实体类和结果集之间的映射关系
                    //获取bean的属性
                    PropertyDescriptor[] propertyDescriptors;
                    try {
                        propertyDescriptors = Introspector.getBeanInfo(beanClass).getPropertyDescriptors();
                    } catch (IntrospectionException e) {
                        throw new SqlExImpossibleException("无法获取实体类(" + beanClass.getName() + ")的属性");
                    }
                    //解析bean的属性(不填充column index的值)
                    propertyInfo = Arrays.stream(propertyDescriptors)
                            .map(p -> {
                                Method writeMethod = p.getWriteMethod();
                                if (writeMethod != null && p.getReadMethod() != null) {
                                    SqlExColumnName columnNameAnnotation = writeMethod.getAnnotation(SqlExColumnName.class);
                                    if (columnNameAnnotation != null)
                                        //如果指定了列名则使用列名
                                        return new PropertyInfo(p.getName(), columnNameAnnotation.value(), writeMethod, p.getPropertyType());
                                    else
                                        //如果没有指定列名,则使用属性名
                                        return new PropertyInfo(p.getName(), p.getName(), writeMethod, p.getPropertyType());
                                }
                                return null;
                            })
                            .filter(Objects::nonNull)
                            .toArray(PropertyInfo[]::new);
                    //获取result set的元数据
                    ResultSetMetaData metaData = resultSet.getMetaData();
                    //填充columnIndex的值,从而建立和result set的对应关系
                    for (PropertyInfo property : propertyInfo) {
                        //遍历result set的column
                        //优先全等匹配
                        for (int colIndex = 1; colIndex <= metaData.getColumnCount(); colIndex++) {
                            if (property.columnName.equals(metaData.getColumnLabel(colIndex))) {
                                property.columnIndex = colIndex;
                                break;
                            }
                        }
                        //如果全等匹配没有找到,则忽略大小写匹配
                        for (int colIndex = 1; colIndex <= metaData.getColumnCount(); colIndex++) {
                            if (property.columnName.equalsIgnoreCase(metaData.getColumnLabel(colIndex))) {
                                property.columnIndex = colIndex;
                                break;
                            }
                        }
                        //找不到
                        if (property.columnIndex <= 0)
                            throw new SqlExImpossibleException("实体类 " + beanClass.getSimpleName() + " 的 " + property.name + " 属性无法在结果集中找到对应的 " + property.columnName + " 列数据");
                    }

                    //缓存起来
                    this.beanPropertyInfoCaches = propertyInfo;
                }
            }
        }
        return propertyInfo;
    }


    //从结果集中获取实体
    public List<T> fetch(ResultSet resultSet) throws SQLException {
        //结果列表
        LinkedList<T> resultList = new LinkedList<>();

        //获取实体的属性信息
        PropertyInfo[] propertyInfos = getPropertyInfo(resultSet);

        while (resultSet.next()) {
            //新建实体类实例
            T beanInstance;
            try {
                beanInstance = beanConstructor.newInstance();
            } catch (Exception e) {
                throw new SqlExImpossibleException("无法创建实体类(" + beanClass.getName() + ")的实例");
            }
            //填充实体类的属性值
            for (PropertyInfo propertyInfo : propertyInfos) {
                //对应的列索引
                int colIndex = propertyInfo.columnIndex;
                //从result set获取的值
                Object value = fetchColumn(resultSet, colIndex, propertyInfo.dataType);
                //写入数据
                try {
                    propertyInfo.writeMethod.invoke(beanInstance, value);
                } catch (Exception e) {
                    throw new SqlExImpossibleException("无法向实体类(" + beanClass.getName() + ")中写入属性(" + propertyInfo.name + ")的值");
                }
            }

            //添加到结果列表
            resultList.add(beanInstance);
        }
        return resultList;
    }
}
