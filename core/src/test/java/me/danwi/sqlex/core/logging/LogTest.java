package me.danwi.sqlex.core.logging;

import org.junit.jupiter.api.Test;

/**
 * @author wjy
 */
public class LogTest {

    @Test
    public void test() {
        final Log log = LogFactory.getLog(LogTest.class);

        log.info("test");
        log.debug("test debug");
        log.error("test error", new LogException("test error"));
    }
}
