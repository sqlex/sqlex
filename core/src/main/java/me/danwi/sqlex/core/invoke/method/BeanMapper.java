package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;

import java.beans.BeanInfo;
import java.beans.IntrospectionException;
import java.beans.Introspector;
import java.beans.PropertyDescriptor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.math.BigDecimal;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.util.HashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;

public class BeanMapper {
    private final PropertyDescriptor[] propertyDescriptors;

    private final Class<?> beanClass;

    BeanMapper(Class<?> bean) throws IntrospectionException {
        beanClass = bean;
        BeanInfo beanInfo = Introspector.getBeanInfo(bean);
        propertyDescriptors = beanInfo.getPropertyDescriptors();
    }

    public List<?> fetch(ResultSet resultSet) throws SQLException {
        LinkedList<Object> resultList = new LinkedList<>();

        //找到属性对应列
        ResultSetMetaData metaData = resultSet.getMetaData();
        Map<String, Integer> map = new HashMap<>();
        for (PropertyDescriptor propertyDescriptor : propertyDescriptors) {
            for (int i = 1; i <= metaData.getColumnCount(); i++) {
                String columnLabel = metaData.getColumnLabel(i);
                if (propertyDescriptor.getName().equals(columnLabel) && propertyDescriptor.getWriteMethod() != null) {
                    map.put(columnLabel, i);
                }
            }
        }

        while (resultSet.next()) {
            try {
                Object beanInstance = beanClass.getDeclaredConstructor().newInstance();

                for (PropertyDescriptor propertyDescriptor : propertyDescriptors) {
                    Method writeMethod = propertyDescriptor.getWriteMethod();
                    if (writeMethod == null)
                        continue;
                    Integer index = map.get(propertyDescriptor.getName());
                    if (index == null)
                        throw new SqlExImpossibleException("属性在结果集中找不到对应的列");
                    String typeName = propertyDescriptor.getPropertyType().getName();
                    Object value;
                    switch (typeName) {
                        case "java.lang.Boolean":
                            value = resultSet.getBoolean(index);
                            break;
                        case "java.lang.Integer":
                            value = resultSet.getInt(index);
                            break;
                        case "java.lang.Long":
                            value = resultSet.getLong(index);
                            break;
                        case "java.lang.Float":
                            value = resultSet.getFloat(index);
                            break;
                        case "java.lang.Double":
                            value = resultSet.getDouble(index);
                            break;
                        case "java.math.BigDecimal":
                            value = resultSet.getBigDecimal(index);
                            break;
                        case "java.math.BigInteger":
                            BigDecimal decimal = resultSet.getBigDecimal(index);
                            value = (decimal == null ? null : decimal.toBigInteger());
                            break;
                        case "java.lang.String":
                            value = resultSet.getString(index);
                            break;
                        case "java.sql.Date":
                            value = resultSet.getDate(index);
                            break;
                        case "java.sql.Time":
                            value = resultSet.getTime(index);
                            break;
                        case "java.sql.Timestamp":
                            value = resultSet.getTimestamp(index);
                            break;
                        default:
                            throw new Exception("结果类中包含不支持的数据类型: " + typeName);
                    }
                    if (resultSet.wasNull())
                        value = null;
                    writeMethod.invoke(beanInstance, value);
                }

                resultList.add(beanInstance);
            } catch (Exception e) {
                throw new SqlExImpossibleException(e.getMessage());
            }
        }
        return resultList;
    }
}
