package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.jdbc.mapper.BasicTypeMapper;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Method;
import java.sql.*;
import java.util.List;

public class InsertMethodProxy extends BaseMethodProxy {
    private final BasicTypeMapper generatedColumnMapper;

    public InsertMethodProxy(Method method, TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        super(method, transactionManager, parameterSetter, translator);
        //获取方法的返回值,如果是void,则不需要获取生成列
        Class<?> returnType = method.getReturnType();
        if (returnType.isPrimitive() && returnType.getSimpleName().equals("void"))
            generatedColumnMapper = null;
        else
            generatedColumnMapper = new BasicTypeMapper(returnType);
    }

    @Override
    protected Object invoke(Object[] args, Connection connection) throws SQLException {
        String sql = rewriteSQL(args);
        try (PreparedStatement statement = connection.prepareStatement(sql, Statement.RETURN_GENERATED_KEYS)) {
            //设置预处理语句参数
            List<Object> reorderArgs = reorderArgs(args);
            parameterSetter.setParameters(statement, reorderArgs);
            //执行
            statement.executeUpdate();
            //获取生成列的值
            try (ResultSet rs = statement.getGeneratedKeys()) {
                //如果存在生成列,则获取它的值
                if (this.generatedColumnMapper != null) {
                    List<?> fetchResult = this.generatedColumnMapper.fetch(rs);
                    if (fetchResult.size() > 0)
                        return fetchResult.get(0);
                }
            }
            //没有生成列信息,则返回null
            return null;
        }
    }
}
