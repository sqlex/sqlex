package me.danwi.sqlex.core.query;

import java.util.List;

public class SQLParameterBind {
    private final String SQL;
    private final List<Object> parameters;

    public SQLParameterBind(String sql, List<Object> parameters) {
        SQL = sql;
        this.parameters = parameters;
    }

    public String getSQL() {
        return SQL;
    }

    public List<Object> getParameters() {
        return parameters;
    }
}
