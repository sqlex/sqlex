package me.danwi.sqlex.core.annotation;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
public @interface SqlExConverters {
    SqlExConverter[] value();
}
