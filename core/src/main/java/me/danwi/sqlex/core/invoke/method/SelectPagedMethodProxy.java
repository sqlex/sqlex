package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.TransactionManager;
import me.danwi.sqlex.core.type.PagedResult;

import java.lang.reflect.Method;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.util.Arrays;
import java.util.List;

public class SelectPagedMethodProxy extends SelectMethodProxy {
    public SelectPagedMethodProxy(Method method, TransactionManager transactionManager, ParameterConverterRegistry registry) throws SqlExImpossibleException {
        super(method, transactionManager, registry);
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws Exception {
        //不带分页参数的参数
        Object[] argsWithoutPage = Arrays.copyOfRange(args, 0, args.length - 2);
        //分页参数
        long pageSize = (long) args[args.length - 2];
        long pageNo = (long) args[args.length - 1];
        //获取重写的语句
        String sql = rewriteSQL(argsWithoutPage);
        //count语句
        String countSQL = "select count(1) from (" + sql + ") temp";
        long total;
        //查询总行数
        try (PreparedStatement statement = connection.prepareStatement(countSQL)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(argsWithoutPage);
            setParameters(statement, reorderArgs);
            //获取到返回值
            try (ResultSet rs = statement.executeQuery()) {
                if (!rs.next())
                    throw new SqlExImpossibleException("无法获取分页总行数");
                total = rs.getLong(1);
            }
        }
        //获取分页结果的SQL
        String pageSQL = "select * from (" + sql + ") temp limit " + pageSize + " offset " + pageSize * pageNo;
        Object result;
        try (PreparedStatement statement = connection.prepareStatement(pageSQL)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(argsWithoutPage);
            setParameters(statement, reorderArgs);
            //获取到返回值
            try (ResultSet rs = statement.executeQuery()) {
                result = getBeanMapper().fetch(rs);
            }
        }

        //noinspection unchecked
        return new PagedResult<>(pageSize, pageNo, total, (List<Object>) result);
    }
}
