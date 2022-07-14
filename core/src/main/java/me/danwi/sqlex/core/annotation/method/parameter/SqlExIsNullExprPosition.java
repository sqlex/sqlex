package me.danwi.sqlex.core.annotation.method.parameter;

import java.lang.annotation.*;

@Target(ElementType.METHOD)
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(SqlExIsNullExprPositions.class)
public @interface SqlExIsNullExprPosition {
    boolean not();

    int marker();

    int start();

    int end();
}
