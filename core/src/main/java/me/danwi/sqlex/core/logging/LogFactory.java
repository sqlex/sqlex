package me.danwi.sqlex.core.logging;

import me.danwi.sqlex.core.logging.slf4j.Slf4jImpl;
import me.danwi.sqlex.core.logging.stdout.StdOutImpl;

import java.lang.reflect.Constructor;

/**
 * SqlEx 日志工厂，SqlEx框架内部使用
 */
public final class LogFactory {

    private static Constructor<? extends Log> logConstructor;

    static {
        tryImplementation(LogFactory::useSlf4jLogging);
        tryImplementation(LogFactory::useStdOutLogging);
    }

    public static Log getLog(Class<?> clazz) {
        return getLog(clazz.getName());
    }

    public static Log getLog(String logger) {
        try {
            return logConstructor.newInstance(logger);
        } catch (Throwable t) {
            throw new LogException("错误创建 logger： " + logger + ".  由于: " + t, t);
        }
    }

    public static synchronized void useSlf4jLogging() {
        setImplementation(Slf4jImpl.class);
    }

    public static synchronized void useStdOutLogging() {
        setImplementation(StdOutImpl.class);
    }

    public static void tryImplementation(Runnable runnable) {
        if (logConstructor == null) {
            try {
                runnable.run();
            } catch (Throwable t) {
                // ignore
            }
        }
    }

    public static void setImplementation(Class<? extends Log> implClass) {
        try {
            Constructor<? extends Log> candidate = implClass.getConstructor(String.class);
            Log log = candidate.newInstance(LogFactory.class.getName());
            log.debug("使用 '" + implClass + "' 适配器 初始化 日志工厂");
            logConstructor = candidate;
        } catch (Throwable t) {
            throw new LogException("错误创建日志工厂实现.  由于: " + t, t);
        }
    }
}
