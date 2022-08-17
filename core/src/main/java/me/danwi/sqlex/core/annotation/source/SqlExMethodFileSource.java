package me.danwi.sqlex.core.annotation.source;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.SOURCE)
@Repeatable(SqlExMethodFileSources.class)
public @interface SqlExMethodFileSource {
    String relativePath();

    String content();
}
