package me.danwi.sqlex.core.annotation;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(SqlExConverters.class)
public @interface SqlExConverter {
    int order();

    Class<?> converter();
}
