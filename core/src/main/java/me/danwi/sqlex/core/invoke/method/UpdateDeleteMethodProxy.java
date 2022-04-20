package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Method;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.util.List;

public class UpdateDeleteMethodProxy extends BaseMethodProxy {
    public UpdateDeleteMethodProxy(Method method, TransactionManager transactionManager, ParameterConverterRegistry registry) {
        super(method, transactionManager, registry);
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws Exception {
        String sql = rewriteSQL(args);
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(args);
            setParameters(statement, reorderArgs);
            return statement.executeLargeUpdate();
        }
    }
}
