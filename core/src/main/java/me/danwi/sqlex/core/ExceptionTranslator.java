package me.danwi.sqlex.core;

/**
 * 异常翻译
 */
@FunctionalInterface
public interface ExceptionTranslator {
    RuntimeException translate(Exception ex);
}
