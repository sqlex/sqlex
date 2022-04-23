package me.danwi.sqlex.core.invoke.method;

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
import java.util.LinkedList;
import java.util.List;

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
        public String name;//属性名
        public Method writeMethod;  //写入方法
        public String dataTypeName; //数据类型名

        public PropertyInfo(String name, Method writeMethod, String dataTypeName) {
            this.name = name;
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
                    //获取result列对应的在bean中对应的write方法
                    //获取result set的元数据
                    ResultSetMetaData metaData = resultSet.getMetaData();
                    //获取bean的属性
                    PropertyDescriptor[] propertyDescriptors;
                    try {
                        propertyDescriptors = Introspector.getBeanInfo(beanClass).getPropertyDescriptors();
                    } catch (IntrospectionException e) {
                        throw new SqlExImpossibleException("无法获取实体类(" + beanClass.getName() + ")的属性");
                    }
                    //新建数组
                    propertyInfo = new PropertyInfo[metaData.getColumnCount()];

                    for (int colIndex = 1; colIndex <= metaData.getColumnCount(); colIndex++) {
                        String columnLabel = metaData.getColumnLabel(colIndex);
                        for (PropertyDescriptor propertyDescriptor : propertyDescriptors) {
                            Method writeMethod = propertyDescriptor.getWriteMethod();
                            if (propertyDescriptor.getName().equals(columnLabel) && writeMethod != null) {
                                propertyInfo[colIndex - 1] = new PropertyInfo(propertyDescriptor.getName(), writeMethod, propertyDescriptor.getPropertyType().getName());
                                break;
                            }
                        }
                        //找不到
                        if (propertyInfo[colIndex - 1] == null)
                            throw new SqlExImpossibleException("结果集中的列(" + columnLabel + ")在实体中无法找到对应的属性");
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
                    case "java.sql.Date":
                        value = resultSet.getDate(colIndex);
                        break;
                    case "java.sql.Time":
                        value = resultSet.getTime(colIndex);
                        break;
                    case "java.sql.Timestamp":
                        value = resultSet.getTimestamp(colIndex);
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
