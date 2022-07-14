package me.danwi.sqlex.core.annotation.repository;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(SqlExTableInfos.class)
public @interface SqlExTableInfo {
    String name();

    String[] columnNames();

    int[] columnTypeIds();

    String[] columnTypeNames();

    long[] columnLengths();

    boolean[] columnUnsigneds();
}
