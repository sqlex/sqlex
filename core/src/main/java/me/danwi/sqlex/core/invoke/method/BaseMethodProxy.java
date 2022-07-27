package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.annotation.method.SqlExScript;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExInExprPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExIsNullExprPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExMarkerPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExParameterPosition;
import me.danwi.sqlex.core.jdbc.ParameterSetter;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.lang.reflect.Method;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.LinkedList;
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
    protected final ParameterSetter parameterSetter;
    //异常翻译
    private final ExceptionTranslator translator;

    private static class MarkerInfo {
        public int argIndex; //引用方法参数的位置
        public SqlExInExprPosition inExprPosition; //?是否在一个in(?)表达式中
        public SqlExIsNullExprPosition isNullExprPosition; //?是否在? is null表达式中
    }

    public BaseMethodProxy(Method method, TransactionManager transactionManager, ParameterSetter parameterSetter, ExceptionTranslator translator) {
        this.transactionManager = transactionManager;
        this.translator = translator;
        //获取sql
        sql = method.getAnnotation(SqlExScript.class).value();
        this.parameterSetter = parameterSetter;

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
        //处理重写信息
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
