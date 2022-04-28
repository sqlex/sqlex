package me.danwi.sqlex.core.annotation;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(SqlExTableColumns.class)
public @interface SqlExTableColumn {
    String tableName();

    String columnName();

    String columnType();

    String columnLength();

    String columnUnsigned();
}
