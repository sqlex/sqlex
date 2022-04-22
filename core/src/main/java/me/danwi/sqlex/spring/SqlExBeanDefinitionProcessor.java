package me.danwi.sqlex.spring;

import me.danwi.sqlex.core.RepositoryLike;
import me.danwi.sqlex.core.annotation.SqlExRepository;
import org.springframework.beans.BeansException;
import org.springframework.beans.factory.FactoryBean;
import org.springframework.beans.factory.annotation.AnnotatedBeanDefinition;
import org.springframework.beans.factory.config.BeanDefinitionHolder;
import org.springframework.beans.factory.config.ConfigurableListableBeanFactory;
import org.springframework.beans.factory.config.RuntimeBeanReference;
import org.springframework.beans.factory.support.AbstractBeanDefinition;
import org.springframework.beans.factory.support.BeanDefinitionRegistry;
import org.springframework.beans.factory.support.BeanDefinitionRegistryPostProcessor;
import org.springframework.context.annotation.ClassPathBeanDefinitionScanner;
import org.springframework.core.type.filter.AnnotationTypeFilter;
import org.springframework.util.StringUtils;

import java.util.Set;

public class SqlExBeanDefinitionProcessor implements BeanDefinitionRegistryPostProcessor {
    private final Class<? extends RepositoryLike> repository;
    private final String factoryName;

    public SqlExBeanDefinitionProcessor(Class<? extends RepositoryLike> repository, String factoryName) {
        this.repository = repository;
        this.factoryName = factoryName;
    }

    @Override
    public void postProcessBeanDefinitionRegistry(BeanDefinitionRegistry registry) throws BeansException {
        ClassPathBeanDefinitionScanner scanner = new ClassPathBeanDefinitionScanner(registry, false) {
            @Override
            protected boolean isCandidateComponent(AnnotatedBeanDefinition beanDefinition) {
                return beanDefinition.getMetadata().isInterface() && beanDefinition.getMetadata().isIndependent();
            }

            @Override
            protected Set<BeanDefinitionHolder> doScan(String... basePackages) {
                Set<BeanDefinitionHolder> beanDefinitionHolders = super.doScan(basePackages);

                for (BeanDefinitionHolder holder : beanDefinitionHolders) {
                    AbstractBeanDefinition definition = (AbstractBeanDefinition) holder.getBeanDefinition();
                    String beanClassName = definition.getBeanClassName();
                    assert beanClassName != null;
                    definition.setBeanClass(SqlExFactoryBean.class);
                    definition.getConstructorArgumentValues().addGenericArgumentValue(beanClassName);
                    if (StringUtils.hasText(factoryName))
                        definition.getPropertyValues().addPropertyValue("factory", new RuntimeBeanReference(factoryName));
                    definition.setAttribute(FactoryBean.OBJECT_TYPE_ATTRIBUTE, beanClassName);
                    definition.setAutowireMode(AbstractBeanDefinition.AUTOWIRE_BY_TYPE);
                    definition.setRole(AbstractBeanDefinition.ROLE_INFRASTRUCTURE);
                    definition.setAutowireCandidate(true);
                    definition.setLazyInit(false);
                }

                return beanDefinitionHolders;
            }
        };
        scanner.resetFilters(false);
        scanner.addIncludeFilter(new AnnotationTypeFilter(SqlExRepository.class));
        scanner.scan(repository.getPackage().getName());
    }

    @Override
    public void postProcessBeanFactory(ConfigurableListableBeanFactory beanFactory) throws BeansException {

    }
}
