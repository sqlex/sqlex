package me.danwi.sqlex.core.invoke.setter;

import java.sql.PreparedStatement;
import java.sql.SQLException;

public interface ParameterSetter {
    void setParameters(PreparedStatement statement, Object[] args) throws SQLException;
}
