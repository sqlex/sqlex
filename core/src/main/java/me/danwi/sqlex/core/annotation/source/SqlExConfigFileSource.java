package me.danwi.sqlex.core.annotation.source;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.SOURCE)
public @interface SqlExConfigFileSource {
    String value();
}
