package me.danwi.sqlex.spring;

import me.danwi.sqlex.core.DaoFactory;
import org.springframework.beans.factory.FactoryBean;

public class SqlExFactoryBean<T> implements FactoryBean<T> {
    private Class<T> daoInterface;

    private DaoFactory factory;

    public SqlExFactoryBean(Class<T> clazz) {
        this.daoInterface = clazz;
    }

    public void setFactory(DaoFactory factory) {
        this.factory = factory;
    }

    @Override
    public T getObject() throws Exception {
        if (this.factory == null)
            throw new Exception("请确保容器中注册有DaoFactory");
        //noinspection unchecked
        return this.factory.getInstance(this.daoInterface);
    }

    @Override
    public Class<T> getObjectType() {
        return daoInterface;
    }

    @Override
    public boolean isSingleton() {
        return true;
    }
}
