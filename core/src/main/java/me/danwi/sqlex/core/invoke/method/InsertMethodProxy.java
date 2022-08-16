package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

import java.lang.reflect.Method;

public class InsertMethodProxy extends BaseMethodProxy {
    private final Class<?> returnType;

    public InsertMethodProxy(Method method, RawSQLExecutor executor) {
        super(method, executor);
        //获取方法的返回值,如果是void,则不需要获取生成列
        Class<?> returnType = method.getReturnType();
        if (returnType.isPrimitive() && returnType.getSimpleName().equals("void"))
            this.returnType = null;
        else
            this.returnType = returnType;
    }

    @Override
    public Object invoke(Object[] args) {
        return this.executor.execute(this.returnType, rewriteSQL(args), reorderArgs(args)).getGeneratedKey();
    }
}
