package me.danwi.sqlex.core.annotation.repository;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(SqlExSchemas.class)
public @interface SqlExSchema {
    int version();

    String[] scripts();
}
