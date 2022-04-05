package me.danwi.sqlex.core;

import me.danwi.sqlex.core.annotation.*;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;

public class DataSource<R extends RepositoryLike> {
    private String replace(String str, int start, int end, String newStr) {
        return str.substring(0, start) + newStr + str.substring(end);
    }

    public <D extends R> D getInstance(Class<D> clazz) {
        InvocationHandler handler = new InvocationHandler() {
            @Override
            public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
                //获取sql语句
                String sql = method.getDeclaredAnnotation(SqlExScript.class).value();
                //获取预处理参数映射位置
                int[] markerPositions = method.getDeclaredAnnotation(SqlExMarkerPosition.class).value();
                //获取参数的映射关系
                int[] parameterPositions = method.getDeclaredAnnotation(SqlExParameterPosition.class).value();
                //获取in list表达式的位置
                SqlExInExprPosition[] inExprPositions = method.getDeclaredAnnotationsByType(SqlExInExprPosition.class);

                //SQL预处理参数列表
                ArrayList<Object> sqlArgs = new ArrayList();
                //挨个处理参数
                for (int index = 0; index < markerPositions.length; index++) {
                    //?所在的位置
                    int markerPosition = markerPositions[index];
                    //判断这个参数是否需要展开
                    SqlExInExprPosition inExprPosition = null;
                    for (SqlExInExprPosition position : inExprPositions) {
                        if (position.marker() == markerPosition) {
                            inExprPosition = position;
                            break;
                        }
                    }
                    //参数位置映射
                    Object methodArg = args[parameterPositions[index]];
                    //如果是一个in list的预处理参数,则需要做进一步的优化
                    if (inExprPosition != null) {
                        //展开/替换in语句
                        if (methodArg == null) {
                            //替换in语句
                            sql = replace(sql, inExprPosition.start(), inExprPosition.end(), inExprPosition.not() ? "1=1" : "1=2");
                        } else {
                            //TODO: 可能还有数组, Iterable?
                            if (methodArg instanceof List) {
                                //集合参数,需要扩展或者改写sql
                                List<?> listArg = (List<?>) methodArg;
                                if (listArg.size() == 0) {
                                    //替换in语句
                                    sql = replace(sql, inExprPosition.start(), inExprPosition.end(), inExprPosition.not() ? "1=1" : "1=2");
                                } else {
                                    //扩展?为多个?
                                    String markerPart = listArg.stream()
                                            .map(it -> "?")
                                            .collect(Collectors.joining(","));
                                    sql = replace(sql, inExprPosition.marker(), inExprPosition.marker() + 1, markerPart);
                                    sqlArgs.addAll(listArg);
                                }
                            } else {
                                //单个参数,可以认为 in (单个)
                                sqlArgs.add(methodArg);
                            }
                        }
                    } else {
                        //其他情况
                        sqlArgs.add(methodArg);
                    }
                }

                //模拟执行
                System.out.println("------------SQL-------------");
                System.out.println(sql.trim());
                System.out.println("-----------PARAMS-----------");
                System.out.println("[ " + sqlArgs.stream().map(it -> {
                    if (it == null)
                        return "null";
                    return it.toString();
                }).collect(Collectors.joining(", ")) + " ]");

                //感知返回类型
                Class<?> returnType = (Class<?>) ((ParameterizedType) method.getGenericReturnType()).getActualTypeArguments()[0];
                List<Method> getterMethod = Arrays.stream(returnType.getDeclaredMethods())
                        .filter(it -> it.getName().startsWith("get"))
                        .collect(Collectors.toList());

                List<String> names = getterMethod.stream()
                        .map(Method::getName)
                        .map(it -> it.substring(3))
                        .map(it -> it.substring(0, 1).toLowerCase() + (it.length() > 1 ? it.substring(1) : ""))
                        .collect(Collectors.toList());

                List<String> types = getterMethod.stream()
                        .map(Method::getReturnType)
                        .map(Class::getName)
                        .collect(Collectors.toList());
                System.out.println("-----------RETURN-----------");
                for (int i = 0; i < names.size(); i++)
                    System.out.println(names.get(i) + " : " + types.get(i));
                System.out.println("----------------------------");

                //TODO: 暂时不返回结果,因为没有真正执行语句
                return new ArrayList();
            }
        };

        return (D) Proxy.newProxyInstance(
                clazz.getClassLoader(),
                new Class[]{clazz},
                handler
        );
    }
}
