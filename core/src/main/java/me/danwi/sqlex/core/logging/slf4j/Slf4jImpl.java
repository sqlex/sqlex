package me.danwi.sqlex.core.logging.slf4j;

import me.danwi.sqlex.core.logging.Log;
import me.danwi.sqlex.core.logging.LogException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.slf4j.helpers.NOPLogger;

/**
 * Slf4j实现
 */
public class Slf4jImpl implements Log {

    private Logger logger;

    public Slf4jImpl(String clazz) {
        logger = LoggerFactory.getLogger(clazz);
        if (logger instanceof NOPLogger) {
            throw new LogException("无法获取有效Slf4j日志实例");
        }
    }

    @Override
    public boolean isDebugEnabled() {
        return logger.isDebugEnabled();
    }

    @Override
    public boolean isTraceEnabled() {
        return logger.isTraceEnabled();
    }

    @Override
    public void info(String msg) {
        logger.info(msg);
    }

    @Override
    public void error(String s, Throwable e) {
        logger.error(s, e);
    }

    @Override
    public void error(String s) {
        logger.error(s);
    }

    @Override
    public void debug(String s) {
        logger.debug(s);
    }

    @Override
    public void trace(String s) {
        logger.trace(s);
    }

    @Override
    public void warn(String s) {
        logger.warn(s);
    }
}
