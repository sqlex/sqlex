package me.danwi.sqlex.core.annotation;

import me.danwi.sqlex.core.RepositoryLike;

import java.lang.annotation.*;

@Target(ElementType.TYPE)
@Retention(RetentionPolicy.RUNTIME)
@Inherited
public @interface SqlExRepository {
    Class<? extends RepositoryLike> value();
}
