package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.annotation.method.SqlExScript;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExInExprPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExIsNullExprPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExMarkerPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExParameterPosition;
import me.danwi.sqlex.core.exception.SqlExImpossibleException;
import me.danwi.sqlex.core.repository.ParameterConverterRegistry;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;
import me.danwi.sqlex.core.type.ParameterConverter;

import java.lang.reflect.Method;
import java.math.BigDecimal;
import java.sql.*;
import java.util.*;
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
    //异常翻译
    private final ExceptionTranslator translator;

    private static class MarkerInfo {
        public int argIndex; //引用方法参数的位置
        public SqlExInExprPosition inExprPosition; //?是否在一个in(?)表达式中
        public SqlExIsNullExprPosition isNullExprPosition; //?是否在? is null表达式中
    }

    public BaseMethodProxy(Method method, TransactionManager transactionManager, ParameterConverterRegistry registry, ExceptionTranslator translator) {
        this.transactionManager = transactionManager;
        this.translator = translator;
        //获取sql
        sql = method.getAnnotation(SqlExScript.class).value();
        this.registry = registry;

        //填充marker信息
        int[] markerPositions = method.getAnnotation(SqlExMarkerPosition.class).value();
        int[] parameterPositions = method.getAnnotation(SqlExParameterPosition.class).value();
        SqlExInExprPosition[] inExprPositions = method.getAnnotationsByType(SqlExInExprPosition.class);
        SqlExIsNullExprPosition[] isNullExprPositions = method.getAnnotationsByType(SqlExIsNullExprPosition.class);
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
            //这个?是否在is null表达式中
            for (SqlExIsNullExprPosition isNullExprPosition : isNullExprPositions) {
                if (isNullExprPosition.marker() == sqlIndex) {
                    markerInfo.isNullExprPosition = isNullExprPosition;
                    break;
                }
            }
            //?对应方法的参数索引
            markerInfo.argIndex = parameterPositions[index];

            markerInfos[index] = markerInfo;
        }
    }

    //设置单个参数 TODO: 部分数据类型没有补全
    private void setParameter(PreparedStatement statement, int index, Object arg) throws SQLException {
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
        } else if (arg instanceof byte[]) { //TODO: 尚未确认
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
            statement.setTimestamp(index, new java.sql.Timestamp(((java.util.Date) arg).getTime()));
            return;
        } else if (arg instanceof java.time.LocalDate ||
                arg instanceof java.time.LocalTime ||
                arg instanceof java.time.LocalDateTime ||
                arg instanceof java.time.OffsetTime ||
                arg instanceof java.time.OffsetDateTime ||
                arg instanceof java.time.ZonedDateTime
        ) {
            statement.setObject(index, arg);
            return;
        } else if (arg instanceof java.time.Instant) {
            statement.setTimestamp(index, Timestamp.from((java.time.Instant) arg));
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

            //如果参数在is null中,其直接会被作为常量折叠,所以不用传入
            if (markerInfo.isNullExprPosition != null) {
                continue;
            }

            //其他情况,直接添加为参数即可
            reorderArgs.add(methodArg);
        }
        return reorderArgs;
    }

    private static class RewriteInfo {
        public int start;
        public int end;
        public String content;

        public RewriteInfo(int start, int end, String content) {
            this.start = start;
            this.end = end;
            this.content = content;
        }
    }

    /**
     * 根据方法调用时的参数来重写SQL
     *
     * @param methodArgs 方法被调用时传入的参数
     * @return 被重写的SQL
     */
    protected String rewriteSQL(Object[] methodArgs) {
        List<RewriteInfo> rewriteInfos = new LinkedList<>();
        //构造重写信息
        for (MarkerInfo markerInfo : markerInfos) {
            //获取到方法调用时的参数
            Object methodArg = methodArgs[markerInfo.argIndex];

            //是否在一个in当中
            if (markerInfo.inExprPosition != null) {
                //如果参数为空
                if (methodArg == null) {
                    //替换in语句
                    rewriteInfos.add(
                            new RewriteInfo(
                                    markerInfo.inExprPosition.start(),
                                    markerInfo.inExprPosition.end(),
                                    markerInfo.inExprPosition.not() ? "1=1" : "1=2"
                            )
                    );
                } else if (methodArg instanceof List) {
                    List<?> listArg = (List<?>) methodArg;
                    if (listArg.size() == 0) {
                        //形同null
                        rewriteInfos.add(
                                new RewriteInfo(
                                        markerInfo.inExprPosition.start(),
                                        markerInfo.inExprPosition.end(),
                                        markerInfo.inExprPosition.not() ? "1=1" : "1=2"
                                )
                        );
                    } else {
                        //拓展?为多个?
                        String markerPart = listArg.stream().map(it -> "?").collect(Collectors.joining(","));
                        rewriteInfos.add(
                                new RewriteInfo(
                                        markerInfo.inExprPosition.marker(),
                                        markerInfo.inExprPosition.marker() + 1,
                                        markerPart
                                )
                        );
                    }
                }
            }

            //是否在is null当中
            if (markerInfo.isNullExprPosition != null) {
                boolean argIsNull = methodArg == null;
                boolean sqlIsNull = !markerInfo.isNullExprPosition.not();
                rewriteInfos.add(
                        new RewriteInfo(
                                markerInfo.isNullExprPosition.start(),
                                markerInfo.isNullExprPosition.end(),
                                argIsNull == sqlIsNull ? "1=1" : "1=2"
                        )
                );
            }
        }
        //处理重写信息 TODO: 还需要计算重写有没有重叠部分,不过在这种编译那边不会出现这种情况
        StringBuilder rewrittenSQL = new StringBuilder(this.sql);
        while (!rewriteInfos.isEmpty()) {
            //取得第一个
            RewriteInfo rewriteInfo = rewriteInfos.remove(0);
            //替换字符串内容
            rewrittenSQL.replace(rewriteInfo.start, rewriteInfo.end, rewriteInfo.content);
            //计算尺寸的增长
            int sizeGrow = rewriteInfo.content.length() - (rewriteInfo.end - rewriteInfo.start);
            //由于改变了原有的字符串,现在需要重新计算接下来的位置信息
            rewriteInfos.forEach(info -> {
                if (info.start >= rewriteInfo.end) {
                    info.start += sizeGrow;
                    info.end += sizeGrow;
                }
            });
        }
        //返回重写后的结果
        return rewrittenSQL.toString();
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
    public Object invoke(Object[] args) {
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
        } catch (SQLException e) {
            throw translator.translate(e);
        } finally {
            //不是事务中的连接主要手动关闭
            if (currentTransaction == null) {
                try {
                    connection.close();
                } catch (SQLException e) {
                    //noinspection ThrowFromFinallyBlock
                    throw translator.translate(e);
                }
            }
        }
    }

    protected abstract Object invoke(Object[] args, Connection connection) throws SQLException;
}
