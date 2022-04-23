package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.annotation.SqlExInExprPosition;
import me.danwi.sqlex.core.annotation.SqlExMarkerPosition;
import me.danwi.sqlex.core.annotation.SqlExParameterPosition;
import me.danwi.sqlex.core.annotation.SqlExScript;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;
import me.danwi.sqlex.core.type.ParameterConverter;

import java.lang.reflect.Method;
import java.math.BigDecimal;
import java.sql.*;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Collectors;

public abstract class BaseMethodProxy implements MethodProxy {
    //事务管理器
    final private TransactionManager transactionManager;
    //SQL语句
    final private String sql;
    //预处理参数信息
    private final MarkerInfo[] markerInfos;
    //参数转换器注册表
    private final ParameterConverterRegistry registry;

    private static class MarkerInfo {
        public int argIndex; //引用方法参数的位置
        public SqlExInExprPosition inExprPosition; //?是否在一个in(?)表达式中
    }

    public BaseMethodProxy(Method method, TransactionManager transactionManager, ParameterConverterRegistry registry) {
        this.transactionManager = transactionManager;
        //获取sql
        sql = method.getAnnotation(SqlExScript.class).value();
        this.registry = registry;

        //填充marker信息
        int[] markerPositions = method.getAnnotation(SqlExMarkerPosition.class).value();
        int[] parameterPositions = method.getAnnotation(SqlExParameterPosition.class).value();
        SqlExInExprPosition[] inExprPositions = method.getAnnotationsByType(SqlExInExprPosition.class);
        markerInfos = new MarkerInfo[markerPositions.length];
        for (int index = 0; index < markerPositions.length; index++) {
            MarkerInfo markerInfo = new MarkerInfo();
            //?所在的位置
            int sqlIndex = markerPositions[index];
            //这个?是否在in表达式中
            for (SqlExInExprPosition inExprPosition : inExprPositions) {
                if (inExprPosition.marker() == sqlIndex) {
                    markerInfo.inExprPosition = inExprPosition;
                    break;
                }
            }
            //?对应方法的参数索引
            markerInfo.argIndex = parameterPositions[index];

            markerInfos[index] = markerInfo;
        }
    }

    //替换字符串
    private String replace(String str, int start, int end, String newStr) {
        return str.substring(0, start) + newStr + str.substring(end);
    }

    //设置单个参数 TODO: 部分数据类型没有补全
    private void setParameter(PreparedStatement statement, int index, Object arg) throws SQLException, SqlExImpossibleException {
        if (arg == null) {
            statement.setNull(index, Types.NULL);
            return;
        } else if (arg instanceof Boolean) {
            statement.setBoolean(index, (Boolean) arg);
            return;
        } else if (arg instanceof Byte) {
            statement.setByte(index, (Byte) arg);
            return;
        } else if (arg instanceof Short) {
            statement.setShort(index, (Short) arg);
            return;
        } else if (arg instanceof Integer) {
            statement.setInt(index, (Integer) arg);
            return;
        } else if (arg instanceof Long) {
            statement.setLong(index, (Long) arg);
            return;
        } else if (arg instanceof Float) {
            statement.setFloat(index, (Float) arg);
            return;
        } else if (arg instanceof Double) {
            statement.setDouble(index, (Double) arg);
            return;
        } else if (arg instanceof Character) {
            statement.setString(index, arg.toString());
            return;
        } else if (arg instanceof String) {
            statement.setString(index, (String) arg);
            return;
        } else if (arg instanceof BigDecimal) {
            statement.setBigDecimal(index, (BigDecimal) arg);
            return;
        } else if (arg instanceof byte[]) {
            statement.setBytes(index, (byte[]) arg);
            return;
        } else if (arg instanceof Blob) {
            statement.setBlob(index, (Blob) arg);
            return;
        } else if (arg instanceof java.sql.Date) {
            statement.setDate(index, (java.sql.Date) arg);
            return;
        } else if (arg instanceof java.sql.Time) {
            statement.setTime(index, (java.sql.Time) arg);
            return;
        } else if (arg instanceof java.sql.Timestamp) {
            statement.setTimestamp(index, (java.sql.Timestamp) arg);
            return;
        } else if (arg instanceof java.util.Date) {
            statement.setTimestamp(index, new Timestamp(((java.util.Date) arg).getTime()));
            return;
        } else {
            ParameterConverter<Object, Object> converter = registry.getConverterFor(arg);
            if (converter != null) {
                Object convertedArg = converter.convert(arg);
                setParameter(statement, index, convertedArg);
                return;
            }
        }
        throw new SqlExImpossibleException("不支持的参数数据类型");
    }

    /**
     * 根据预处理参数重排序方法参数
     *
     * @param methodArgs 方法被调用时传入的参数
     * @return 重排序后的参数
     */
    protected List<Object> reorderArgs(Object[] methodArgs) {
        List<Object> reorderArgs = new ArrayList<>(markerInfos.length);
        for (MarkerInfo markerInfo : markerInfos) {
            //获取到方法调用时的参数
            Object methodArg = methodArgs[markerInfo.argIndex];

            //如果参数在in中
            if (markerInfo.inExprPosition != null) {
                //如果参数为空
                if (methodArg == null)
                    continue;
                //参数是一个list
                if (methodArg instanceof List) {
                    List<?> listArg = (List<?>) methodArg;
                    //如果列表为空,形同null
                    if (listArg.size() == 0)
                        continue;
                    //参数个数大于0,则需要拓展参数列表
                    reorderArgs.addAll(listArg);
                } else {
                    //单个值
                    reorderArgs.add(methodArg);
                }

                continue;
            }

            //其他情况,直接添加为参数即可
            reorderArgs.add(methodArg);
        }
        return reorderArgs;
    }

    /**
     * 根据方法调用时的参数来重写SQL
     *
     * @param methodArgs 方法被调用时传入的参数
     * @return 被重写的SQL
     */
    protected String rewriteSQL(Object[] methodArgs) {
        String rewrittenSQL = this.sql;
        for (MarkerInfo markerInfo : markerInfos) {
            //是否在一个in当中
            if (markerInfo.inExprPosition != null) {
                //获取到方法调用时的参数
                Object methodArg = methodArgs[markerInfo.argIndex];
                //如果参数为空
                if (methodArg == null) {
                    //替换in语句
                    rewrittenSQL = replace(
                            rewrittenSQL,
                            markerInfo.inExprPosition.start(), markerInfo.inExprPosition.end(),
                            markerInfo.inExprPosition.not() ? "1=1" : "1=2"
                    );
                } else if (methodArg instanceof List) {
                    List<?> listArg = (List<?>) methodArg;
                    if (listArg.size() == 0) {
                        //形同null
                        rewrittenSQL = replace(
                                rewrittenSQL,
                                markerInfo.inExprPosition.start(), markerInfo.inExprPosition.end(),
                                markerInfo.inExprPosition.not() ? "1=1" : "1=2"
                        );
                    } else {
                        //拓展?为多个?
                        String markerPart = listArg.stream().map(it -> "?").collect(Collectors.joining(","));
                        rewrittenSQL = replace(rewrittenSQL, markerInfo.inExprPosition.start(), markerInfo.inExprPosition.end(), markerPart);
                    }
                }
            }
        }
        return rewrittenSQL;
    }

    /**
     * 将参数设置到预处理语句
     *
     * @param statement 预处理语句
     * @param args      参数
     * @throws SQLException SQL异常
     */
    protected void setParameters(PreparedStatement statement, List<Object> args) throws SQLException {
        for (int i = 0; i < args.size(); i++) {
            setParameter(statement, i + 1, args.get(i));
        }
    }

    @Override
    public Object invoke(Object[] args) throws SQLException {
        //获取事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        Connection connection;
        if (currentTransaction != null)
            //存在事务,则从事务中获取连接
            connection = currentTransaction.getConnection();
        else
            //不存在事务,则新建一个连接
            connection = transactionManager.newConnection();

        //调用方法
        try {
            return invoke(args, connection);
        } finally {
            //不是事务中的连接主要手动关闭
            if (currentTransaction == null)
                connection.close();
        }
    }

    protected abstract Object invoke(Object[] args, Connection connection) throws SQLException;
}
