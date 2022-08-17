package me.danwi.sqlex.spring;

import me.danwi.sqlex.core.RepositoryLike;
import org.springframework.context.annotation.Import;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.TYPE)
@Import(SqlExImportRegistrar.class)
public @interface ImportSqlEx {
    Class<? extends RepositoryLike> value();

    String factoryName() default "";
}
