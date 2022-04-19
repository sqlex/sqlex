package me.danwi.sqlex.core.invoke.getter;

import java.sql.ResultSet;
import java.sql.SQLException;

public interface ResultGetter {
    Object getResult(ResultSet resultSet) throws SQLException;
}
