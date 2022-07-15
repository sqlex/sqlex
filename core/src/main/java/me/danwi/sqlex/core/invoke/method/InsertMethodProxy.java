package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Method;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.util.List;

public class InsertMethodProxy extends BaseMethodProxy {
    public InsertMethodProxy(Method method, TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        super(method, transactionManager, parameterSetter, translator);
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws SQLException {
        String sql = rewriteSQL(args);
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(args);
            parameterSetter.setParameters(statement, reorderArgs);
            //TODO: 这里暂时返回的是插入的行数,以后要修改成last inserted id
            try {
                return statement.executeLargeUpdate();
            } catch (UnsupportedOperationException e) {
                return (long) statement.executeUpdate();
            }
        }
    }
}
