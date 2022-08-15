package me.danwi.sqlex.core.jdbc.mapper;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.LinkedList;
import java.util.List;

public class BasicTypeMapper<T> extends RowMapper {
    private final Class<?> dataType;


    public BasicTypeMapper(Class<T> dataType) {
        this.dataType = dataType;
    }

    @Override
    public List<T> fetch(ResultSet resultSet) throws SQLException {
        //结果列表
        LinkedList<T> resultList = new LinkedList<>();

        while (resultSet.next()) {
            Object value = fetchColumn(resultSet, 1, dataType);
            resultList.add((T) value);
        }

        return resultList;
    }
}
