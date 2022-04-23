package me.danwi.sqlex.spring;

import org.springframework.beans.factory.support.AbstractBeanDefinition;
import org.springframework.beans.factory.support.BeanDefinitionBuilder;
import org.springframework.beans.factory.support.BeanDefinitionRegistry;
import org.springframework.context.annotation.ImportBeanDefinitionRegistrar;
import org.springframework.core.annotation.AnnotationAttributes;
import org.springframework.core.type.AnnotationMetadata;

public class SqlExImportRegistrar implements ImportBeanDefinitionRegistrar {
    @Override
    public void registerBeanDefinitions(AnnotationMetadata importingClassMetadata, BeanDefinitionRegistry registry) {
        AnnotationAttributes attributes = AnnotationAttributes
                .fromMap(importingClassMetadata.getAnnotationAttributes(ImportSqlEx.class.getName()));
        assert attributes != null;
        Class<?> repository = attributes.getClass("value");
        String factoryName = attributes.getString("factoryName");
        AbstractBeanDefinition definition = BeanDefinitionBuilder.genericBeanDefinition(SqlExBeanDefinitionProcessor.class)
                .addConstructorArgValue(repository)
                .addConstructorArgValue(factoryName)
                .getBeanDefinition();
        registry.registerBeanDefinition(repository.getPackage().getName() + "#BeanImport", definition);
    }
}
