package me.danwi.sqlex.core.annotation;

import java.lang.annotation.*;

@Target(ElementType.METHOD)
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(SqlExLimitPositions.class)
public @interface SqlExLimitPosition {
    boolean hasOffset();

    int count();

    int offset();
}
