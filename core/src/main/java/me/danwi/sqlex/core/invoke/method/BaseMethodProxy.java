package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.common.StringUtils;
import me.danwi.sqlex.core.annotation.method.SqlExScript;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExInExprPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExIsNullExprPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExMarkerPosition;
import me.danwi.sqlex.core.annotation.method.parameter.SqlExParameterPosition;
import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.LinkedList;
import java.util.List;
import java.util.stream.Collectors;

public abstract class BaseMethodProxy implements MethodProxy {
    //SQL语句
    private final String sql;
    //预处理参数信息
    private final MarkerInfo[] markerInfos;
    //SQL执行器
    protected final RawSQLExecutor executor;

    private static class MarkerInfo {
        public int argIndex; //引用方法参数的位置
        public SqlExInExprPosition inExprPosition; //?是否在一个in(?)表达式中
        public SqlExIsNullExprPosition isNullExprPosition; //?是否在? is null表达式中
    }

    public BaseMethodProxy(Method method, RawSQLExecutor executor) {
        this.executor = executor;
        //获取sql
        sql = method.getAnnotation(SqlExScript.class).value();

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

    /**
     * 根据方法调用时的参数来重写SQL
     *
     * @param methodArgs 方法被调用时传入的参数
     * @return 被重写的SQL
     */
    protected String rewriteSQL(Object[] methodArgs) {
        List<StringUtils.ReplaceInfo> replaces = new LinkedList<>();
        //构造重写信息
        for (MarkerInfo markerInfo : markerInfos) {
            //获取到方法调用时的参数
            Object methodArg = methodArgs[markerInfo.argIndex];

            //是否在一个in当中
            if (markerInfo.inExprPosition != null) {
                //如果参数为空
                if (methodArg == null) {
                    //替换in语句
                    replaces.add(
                            new StringUtils.ReplaceInfo(
                                    markerInfo.inExprPosition.start(),
                                    markerInfo.inExprPosition.end(),
                                    markerInfo.inExprPosition.not() ? "1=1" : "1=2"
                            )
                    );
                } else if (methodArg instanceof List) {
                    List<?> listArg = (List<?>) methodArg;
                    if (listArg.size() == 0) {
                        //形同null
                        replaces.add(
                                new StringUtils.ReplaceInfo(
                                        markerInfo.inExprPosition.start(),
                                        markerInfo.inExprPosition.end(),
                                        markerInfo.inExprPosition.not() ? "1=1" : "1=2"
                                )
                        );
                    } else {
                        //拓展?为多个?
                        String markerPart = listArg.stream().map(it -> "?").collect(Collectors.joining(","));
                        replaces.add(
                                new StringUtils.ReplaceInfo(
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
                replaces.add(
                        new StringUtils.ReplaceInfo(
                                markerInfo.isNullExprPosition.start(),
                                markerInfo.isNullExprPosition.end(),
                                argIsNull == sqlIsNull ? "1=1" : "1=2"
                        )
                );
            }
        }
        //重写
        return StringUtils.replace(sql, replaces);
    }
}
