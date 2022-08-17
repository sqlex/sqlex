package me.danwi.sqlex.spring;

import me.danwi.sqlex.core.annotation.repository.SqlExMethods;
import me.danwi.sqlex.core.annotation.repository.SqlExTables;
import org.springframework.beans.factory.FactoryBean;
import org.springframework.beans.factory.config.RuntimeBeanReference;
import org.springframework.beans.factory.support.AbstractBeanDefinition;
import org.springframework.beans.factory.support.BeanDefinitionBuilder;
import org.springframework.beans.factory.support.BeanDefinitionRegistry;
import org.springframework.context.annotation.ImportBeanDefinitionRegistrar;
import org.springframework.core.annotation.AnnotationAttributes;
import org.springframework.core.type.AnnotationMetadata;
import org.springframework.util.StringUtils;

public class SqlExImportRegistrar implements ImportBeanDefinitionRegistrar {
    @Override
    public void registerBeanDefinitions(AnnotationMetadata importingClassMetadata, BeanDefinitionRegistry registry) {
        AnnotationAttributes attributes = AnnotationAttributes
                .fromMap(importingClassMetadata.getAnnotationAttributes(ImportSqlEx.class.getName()));
        assert attributes != null;
        Class<?> repository = attributes.getClass("value");
        String factoryName = attributes.getString("factoryName");
        //获取到这个repository所有的Dao接口
        SqlExMethods methodsAnnotation = repository.getAnnotation(SqlExMethods.class);
        if (methodsAnnotation != null) {
            Class<?>[] daoMethods = methodsAnnotation.value();
            //循环注册所有的bean
            for (Class<?> daoMethod : daoMethods) {
                registerBeanDefinition(registry, daoMethod, factoryName);
            }
        }
        //获取到这个repository所有的表操作类
        SqlExTables tablesAnnotation = repository.getAnnotation(SqlExTables.class);
        if (tablesAnnotation != null) {
            Class<?>[] tableClasses = tablesAnnotation.value();
            //循环注册所有的bean
            for (Class<?> tableClass : tableClasses) {
                registerBeanDefinition(registry, tableClass, factoryName);
            }
        }
    }

    /**
     * 对单个的dao接口/表操作类实施bean注册
     *
     * @param registry    注册表
     * @param clazz       dao接口/表操作类的类型
     * @param factoryName DaoFactory名称(bean名称)
     */
    private void registerBeanDefinition(BeanDefinitionRegistry registry, Class<?> clazz, String factoryName) {
        AbstractBeanDefinition definition = BeanDefinitionBuilder.genericBeanDefinition(SqlExFactoryBean.class).getBeanDefinition();
        definition.getConstructorArgumentValues().addGenericArgumentValue(clazz.getName());
        if (StringUtils.hasText(factoryName))
            definition.getPropertyValues().addPropertyValue("factory", new RuntimeBeanReference(factoryName));
        definition.setAttribute(FactoryBean.OBJECT_TYPE_ATTRIBUTE, clazz.getName());
        definition.setAutowireMode(AbstractBeanDefinition.AUTOWIRE_BY_TYPE);
        definition.setRole(AbstractBeanDefinition.ROLE_INFRASTRUCTURE);
        definition.setAutowireCandidate(true);
        definition.setLazyInit(false);
        registry.registerBeanDefinition(clazz.getName(), definition);
    }
}
