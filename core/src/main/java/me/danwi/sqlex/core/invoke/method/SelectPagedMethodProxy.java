package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.jdbc.RawSQLExecutor;
import me.danwi.sqlex.core.type.PagedResult;

import java.lang.reflect.Method;
import java.util.Arrays;
import java.util.List;

public class SelectPagedMethodProxy extends SelectMethodProxy {
    public SelectPagedMethodProxy(Method method, RawSQLExecutor executor) {
        super(method, executor);
    }

    @Override
    public Object invoke(Object[] args) {
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
        List<Long> countResult = this.executor.query(null, Long.class, countSQL, reorderArgs(argsWithoutPage));
        if (countResult.isEmpty())
            throw new SqlExImpossibleException("无法获取分页总行数");
        total = countResult.get(0);
        //获取分页结果的SQL
        String pageSQL = "select * from (" + sql + ") temp limit " + pageSize + " offset " + pageSize * (pageNo - 1);
        List<?> result = this.executor.query(getRowMapper(), null, pageSQL, reorderArgs(argsWithoutPage));
        //noinspection unchecked
        return new PagedResult<>(pageSize, pageNo, total, (List<Object>) result);
    }
}
