package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;
import org.jetbrains.annotations.NotNull;

import java.lang.reflect.Method;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.util.List;

public class InsertMethodProxy extends BaseMethodProxy {
    public InsertMethodProxy(@NotNull Method method, @NotNull TransactionManager transactionManager, @NotNull ParameterConverterRegistry registry) {
        super(method, transactionManager, registry);
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws Exception {
        String sql = rewriteSQL(args);
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(args);
            setParameters(statement, reorderArgs);
            //TODO: 这里暂时返回的是插入的行数,以后要修改成last inserted id
            return statement.executeLargeUpdate();
        }
    }
}