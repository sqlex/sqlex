package me.danwi.sqlex.core.logging;

/**
 * SqlEx 日志接口
 */
public interface Log {
    boolean isDebugEnabled();

    boolean isTraceEnabled();

    void info(String msg);

    void error(String s, Throwable e);

    void error(String s);

    void debug(String s);

    void trace(String s);

    void warn(String s);
}
