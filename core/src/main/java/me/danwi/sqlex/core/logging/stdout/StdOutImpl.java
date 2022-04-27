package me.danwi.sqlex.core.logging.stdout;

import me.danwi.sqlex.core.logging.Log;

public class StdOutImpl implements Log {

    private String clazz;

    public StdOutImpl(String clazz) {
        this.clazz = clazz;
    }

    @Override
    public boolean isDebugEnabled() {
        return true;
    }

    @Override
    public boolean isTraceEnabled() {
        return true;
    }

    @Override
    public void info(String msg) {
        System.out.println(clazz + ": " + msg);
    }

    @Override
    public void error(String s, Throwable e) {
        if (isTraceEnabled()) {
            System.err.println(clazz + ": " + s);
            e.printStackTrace(System.err);
        }
    }

    @Override
    public void error(String s) {
        if (isTraceEnabled()) {
            System.err.println(clazz + ": " + s);
        }
    }

    @Override
    public void debug(String s) {
        if (isDebugEnabled()) {
            System.out.println(clazz + ": " + s);
        }
    }

    @Override
    public void trace(String s) {
        System.out.println(clazz + ": " + s);
    }

    @Override
    public void warn(String s) {
        System.out.println(clazz + ": " + s);
    }
}
