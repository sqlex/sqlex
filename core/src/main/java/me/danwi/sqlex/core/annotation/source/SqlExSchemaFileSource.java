package me.danwi.sqlex.core.annotation.source;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.SOURCE)
@Repeatable(SqlExSchemaFileSources.class)
public @interface SqlExSchemaFileSource {
    String relativePath();

    String content();
}
