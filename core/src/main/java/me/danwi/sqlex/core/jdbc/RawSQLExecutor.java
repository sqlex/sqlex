package me.danwi.sqlex.core.jdbc;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.mapper.BasicTypeMapper;
import me.danwi.sqlex.core.jdbc.mapper.BeanMapper;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.sql.*;
import java.util.Arrays;
import java.util.List;

/**
 * 原生SQL执行器
 */
public class RawSQLExecutor {
    //参数设置器
    private final ParameterSetter setter;
    //异常翻译器
    private final ExceptionTranslator translator;
    //事务管理器
    private final TransactionManager transactionManager;

    /**
     * 构造一个原生SQL执行器
     * <br>
     * 执行时将从事务管理器中获取连接
     *
     * @param setter             参数设置器
     * @param translator         异常翻译器
     * @param transactionManager 事务管理器
     */
    RawSQLExecutor(ParameterSetter setter, ExceptionTranslator translator, TransactionManager transactionManager) {
        this.setter = setter;
        this.translator = translator;
        this.transactionManager = transactionManager;
    }

    /**
     * 执行SQL,返回执行结果
     *
     * @param generateKeyType 生成键的Java类
     * @param sql             SQL语句
     * @param parameters      预处理参数
     * @param <K>             生成键的Java类型
     * @return 执行结果
     */
    public <K> ExecuteResult<K> execute(Class<K> generateKeyType, String sql, Object... parameters) {
        //尝试获取当前事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        Connection connection;
        if (currentTransaction != null)
            //如果当前存在事务,则从事务中获取连接
            connection = currentTransaction.getConnection();
        else
            //如果当前不存在事务,则新建一个连接
            connection = transactionManager.newConnection();

        //执行SQL
        try (PreparedStatement statement = connection.prepareStatement(sql, Statement.RETURN_GENERATED_KEYS)) {
            //使用参数设置器设置参数
            setter.setParameters(statement, Arrays.asList(parameters));
            //执行,有些驱动可能没有实现executeLargeUpdate
            long affectRows = 0;
            try {
                affectRows = statement.executeLargeUpdate();
            } catch (UnsupportedOperationException e) {
                affectRows = statement.executeUpdate();
            }
            //获取生成列的值
            K generatedKey = null;
            try (ResultSet rs = statement.getGeneratedKeys()) {
                //如果存在生成列,则获取他的值
                BasicTypeMapper<K> mapper = new BasicTypeMapper<>(generateKeyType);
                List<K> fetchResult = mapper.fetch(rs);
                if (fetchResult.size() > 0)
                    generatedKey = fetchResult.get(0);
            }
            //返回结果
            return new ExecuteResult<>(affectRows, generatedKey);
        } catch (SQLException e) {
            throw translator.translate(e);
        } finally {
            //不是事务中的连接需要手动关闭
            if (currentTransaction == null) {
                try {
                    connection.close();
                } catch (SQLException e) {
                    //noinspection ThrowFromFinallyBlock
                    throw translator.translate(e);
                }
            }
        }
    }

    /**
     * 执行SQL查询,返回结果
     *
     * @param beanType   结果映射Bean类
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @param <T>        结果映射Bean类型
     * @return 结果集合
     */
    public <T> List<T> query(Class<T> beanType, String sql, Object... parameters) {
        return query(new BeanMapper<>(beanType), sql, parameters);
    }

    /**
     * 执行SQL查询,返回结果
     *
     * @param beanMapper 结果映射器
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @param <T>        结果映射Bean类型
     * @return 结果集合
     */
    public <T> List<T> query(BeanMapper<T> beanMapper, String sql, Object... parameters) {
        //尝试获取当前事务
        Transaction currentTransaction = transactionManager.getCurrentTransaction();
        Connection connection;
        if (currentTransaction != null)
            //如果当前存在事务,则从事务中获取连接
            connection = currentTransaction.getConnection();
        else
            //如果当前不存在事务,则新建一个连接
            connection = transactionManager.newConnection();

        //执行SQL
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            //使用参数设置器设置参数
            setter.setParameters(statement, Arrays.asList(parameters));
            //获取返回值
            try (ResultSet rs = statement.executeQuery()) {
                return beanMapper.fetch(rs);
            }
        } catch (SQLException e) {
            throw translator.translate(e);
        } finally {
            //不是事务中的连接需要手动关闭
            if (currentTransaction == null) {
                try {
                    connection.close();
                } catch (SQLException e) {
                    //noinspection ThrowFromFinallyBlock
                    throw translator.translate(e);
                }
            }
        }
    }
}
