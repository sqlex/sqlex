package me.danwi.sqlex.core.invoke.method;

import java.sql.SQLException;

public interface MethodProxy {
    Object invoke(Object[] args) throws SQLException;
}
