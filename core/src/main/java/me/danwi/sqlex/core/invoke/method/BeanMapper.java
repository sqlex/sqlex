package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.annotation.SqlExColumnName;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;

import java.beans.IntrospectionException;
import java.beans.Introspector;
import java.beans.PropertyDescriptor;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.math.BigDecimal;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.util.Arrays;
import java.util.LinkedList;
import java.util.List;
import java.util.Objects;

public class BeanMapper {
    //实体类
    private final Class<?> beanClass;
    //实体类构造函数
    private final Constructor<?> beanConstructor;
    //实体类属性信息缓存
    private PropertyInfo[] beanPropertyInfoCaches;

    BeanMapper(Class<?> bean) {
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
        public String dataTypeName; //数据类型名

        public PropertyInfo(String name, String columnName, Method writeMethod, String dataTypeName) {
            this.name = name;
            this.columnName = columnName;
            this.columnIndex = -1;
            this.writeMethod = writeMethod;
            this.dataTypeName = dataTypeName;
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
                                if (writeMethod != null) {
                                    SqlExColumnName columnNameAnnotation = writeMethod.getAnnotation(SqlExColumnName.class);
                                    if (columnNameAnnotation != null)
                                        return new PropertyInfo(p.getName(), columnNameAnnotation.value(), writeMethod, p.getPropertyType().getName());
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
                        for (int colIndex = 1; colIndex <= metaData.getColumnCount(); colIndex++) {
                            if (property.columnName.equals(metaData.getColumnLabel(colIndex))) {
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


    //从结果集中获取实体 TODO: 部分数据类型没有补全
    public List<?> fetch(ResultSet resultSet) throws SQLException {
        //结果列表
        LinkedList<Object> resultList = new LinkedList<>();

        //获取实体的属性信息
        PropertyInfo[] propertyInfos = getPropertyInfo(resultSet);

        while (resultSet.next()) {
            //新建实体类实例
            Object beanInstance;
            try {
                beanInstance = beanConstructor.newInstance();
            } catch (Exception e) {
                throw new SqlExImpossibleException("无法创建实体类(" + beanClass.getName() + ")的实例");
            }

            for (int index = 0; index < propertyInfos.length; index++) {
                PropertyInfo propertyInfo = propertyInfos[index];
                //对应的列索引
                int colIndex = index + 1;
                //从result set获取的值
                Object value;
                switch (propertyInfo.dataTypeName) {
                    case "java.lang.Boolean":
                        value = resultSet.getBoolean(colIndex);
                        break;
                    case "java.lang.Integer":
                        value = resultSet.getInt(colIndex);
                        break;
                    case "java.lang.Long":
                        value = resultSet.getLong(colIndex);
                        break;
                    case "java.lang.Float":
                        value = resultSet.getFloat(colIndex);
                        break;
                    case "java.lang.Double":
                        value = resultSet.getDouble(colIndex);
                        break;
                    case "java.math.BigDecimal":
                        value = resultSet.getBigDecimal(colIndex);
                        break;
                    case "java.math.BigInteger":
                        BigDecimal decimal = resultSet.getBigDecimal(colIndex);
                        value = (decimal == null ? null : decimal.toBigInteger());
                        break;
                    case "java.lang.String":
                        value = resultSet.getString(colIndex);
                        break;
                    case "java.time.LocalDate":
                        value = resultSet.getObject(colIndex, java.time.LocalDate.class);
                        break;
                    case "java.time.LocalTime":
                        value = resultSet.getObject(colIndex, java.time.LocalTime.class);
                        break;
                    case "java.time.LocalDateTime":
                        value = resultSet.getObject(colIndex, java.time.LocalDateTime.class);
                        break;
                    default:
                        throw new SqlExImpossibleException("结果类中包含不支持的数据类型: " + propertyInfo.dataTypeName);
                }
                if (resultSet.wasNull())
                    value = null;
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
