package me.danwi.sqlex.core.annotation.repository;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
public @interface SqlExSchemas {
    SqlExSchema[] value();
}
