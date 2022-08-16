package me.danwi.sqlex.core.invoke.method;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;

import java.lang.reflect.Method;

public class UpdateDeleteMethodProxy extends BaseMethodProxy {
    public UpdateDeleteMethodProxy(Method method, RawSQLExecutor executor) {
        super(method, executor);
    }

    @Override
    public Object invoke(Object[] args) {
        return this.executor.execute(null, rewriteSQL(args), reorderArgs(args)).getAffectRows();
    }
}
