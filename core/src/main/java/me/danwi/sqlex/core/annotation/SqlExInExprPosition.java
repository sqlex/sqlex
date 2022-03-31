package me.danwi.sqlex.core.annotation;

import java.lang.annotation.*;

@Target(ElementType.METHOD)
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(SqlExInExprPositions.class)
public @interface SqlExInExprPosition {
    boolean not();

    int marker();

    int start();

    int end();
}

