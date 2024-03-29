package me.danwi.sqlex.core.jdbc;

import me.danwi.sqlex.common.SQLUtils;
import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.jdbc.mapper.BasicTypeMapper;
import me.danwi.sqlex.core.jdbc.mapper.BeanMapper;
import me.danwi.sqlex.core.jdbc.mapper.RowMapper;
import me.danwi.sqlex.core.transaction.Transaction;
import me.danwi.sqlex.core.transaction.TransactionManager;

import java.sql.*;
import java.util.Arrays;
import java.util.List;
import java.util.Map;

/**
 * 原生SQL执行器
 */
public class RawSQLExecutor {
    //事务管理器
    private final TransactionManager transactionManager;
    //指定使用的连接
    private final Connection connection;
    //参数设置器
    private final ParameterSetter setter;
    //异常翻译器
    private final ExceptionTranslator translator;
    //运行时数据库名称映射
    private final Map<String, String> databaseNameMapping;

    /**
     * 构造一个原生SQL执行器
     * <br>
     * SQL在执行时,将从事务管理器中获取连接
     *
     * @param transactionManager  事务管理器
     * @param setter              参数设置器
     * @param translator          异常翻译器
     * @param databaseNameMapping 数据库名称映射
     */
    public RawSQLExecutor(TransactionManager transactionManager, ParameterSetter setter, ExceptionTranslator translator, Map<String, String> databaseNameMapping) {
        this.transactionManager = transactionManager;
        this.connection = null;
        this.setter = setter;
        this.translator = translator;
        this.databaseNameMapping = databaseNameMapping;
    }

    /**
     * 构造一个原生SQL执行器
     * <br>
     * SQL将在指定的连接上执行
     *
     * @param connection          执行器使用的连接
     * @param setter              参数设置器
     * @param translator          异常翻译器
     * @param databaseNameMapping 数据库名称映射
     */
    public RawSQLExecutor(Connection connection, ParameterSetter setter, ExceptionTranslator translator, Map<String, String> databaseNameMapping) {
        this.transactionManager = null;
        this.connection = connection;
        this.setter = setter;
        this.translator = translator;
        this.databaseNameMapping = databaseNameMapping;
    }

    /**
     * 执行SQL,返回生成键的值
     *
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @return 返回插入的行数
     */
    public long insert(String sql, Object... parameters) {
        return execute(null, sql, Arrays.asList(parameters)).getAffectRows();
    }

    /**
     * 执行SQL,返回生成键的值
     *
     * @param generateKeyType 生成键的Java类
     * @param sql             SQL语句
     * @param parameters      预处理参数
     * @param <K>             生成键的Java类型
     * @return 生成键的值(如果有的化, 没有则为空)
     */
    public <K> K insert(Class<K> generateKeyType, String sql, Object... parameters) {
        return execute(generateKeyType, sql, Arrays.asList(parameters)).getGeneratedKey();
    }

    /**
     * 执行SQL,返回删除的行数
     *
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @return 影响的行数
     */
    public long delete(String sql, Object... parameters) {
        return execute(null, sql, Arrays.asList(parameters)).getAffectRows();
    }

    /**
     * 执行SQL,返回更新的行数
     *
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @return 影响的行数
     */
    public long update(String sql, Object... parameters) {
        return execute(null, sql, Arrays.asList(parameters)).getAffectRows();
    }

    /**
     * 执行SQL查询,返回结果
     *
     * @param rowType    行映射Java类型
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @param <T>        结果映射Bean类型
     * @return 结果集合
     */
    public <T> List<T> select(Class<T> rowType, String sql, Object... parameters) {
        return query(rowType, sql, Arrays.asList(parameters));
    }

    /**
     * 执行SQL,返回影响的行数
     *
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @return 影响的行数
     */
    public long execute(String sql, Object... parameters) {
        return execute(null, sql, Arrays.asList(parameters)).getAffectRows();
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
    public <K> ExecuteResult<K> execute(Class<K> generateKeyType, String sql, List<Object> parameters) {
        //当前执行的连接
        Connection connection;
        //当前事务
        Transaction currentTransaction = null;
        //判断是从事务管理器中获取连接,还是使用现有连接
        if (this.transactionManager != null) {
            //尝试从事务中获取连接
            //尝试获取当前事务
            currentTransaction = transactionManager.getCurrentTransaction();
            if (currentTransaction != null)
                //如果当前存在事务,则从事务中获取连接
                connection = currentTransaction.getConnection();
            else
                //如果当前不存在事务,则新建一个连接
                connection = transactionManager.newConnection();
        } else {
            //否则使用传入的连接
            connection = this.connection;
        }
        //连接必定不为空
        assert connection != null;

        //替换运行时的数据库名称
        sql = SQLUtils.replaceDatabaseName(sql, databaseNameMapping);
        //执行SQL
        try {
            try (PreparedStatement statement = connection.prepareStatement(sql, Statement.RETURN_GENERATED_KEYS)) {
                //使用参数设置器设置参数
                setter.setParameters(statement, parameters);
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
                    if (generateKeyType != null) {
                        //如果存在生成列,则获取他的值
                        BasicTypeMapper<K> mapper = new BasicTypeMapper<>(generateKeyType);
                        List<K> fetchResult = mapper.fetch(rs);
                        if (fetchResult.size() > 0)
                            generatedKey = fetchResult.get(0);
                    }
                }
                //返回结果
                return new ExecuteResult<>(affectRows, generatedKey);
            }
        } catch (SQLException e) {
            throw translator.translate(e);
        } finally {
            //从事务管理器中new出来的连接,需要手动关闭
            if (this.transactionManager != null && currentTransaction == null) {
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
     * 查询SQL,返回查询结果
     *
     * @param rowType    行类型
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @param <T>        行类型
     * @return 结果集
     */
    public <T> List<T> query(Class<T> rowType, String sql, Object... parameters) {
        return query(null, rowType, sql, Arrays.asList(parameters));
    }

    /**
     * 查询SQL,返回查询结果
     *
     * @param rowMapper  行映射器
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @param <T>        行类型
     * @return 结果集
     */
    public <T> List<T> query(RowMapper<T> rowMapper, String sql, Object... parameters) {
        return query(rowMapper, null, sql, Arrays.asList(parameters));
    }

    /**
     * 查询SQL,返回查询结果
     *
     * @param rowMapper  行映射器
     * @param rowType    行类型,与rowMapper二选一
     * @param sql        SQL语句
     * @param parameters 预处理参数
     * @param <T>        行类型
     * @return 结果集
     */
    public <T> List<T> query(RowMapper<T> rowMapper, Class<T> rowType, String sql, List<Object> parameters) {
        //当前执行的连接
        Connection connection;
        //当前事务
        Transaction currentTransaction = null;
        //判断是从事务管理器中获取连接,还是使用现有连接
        if (this.transactionManager != null) {
            //尝试从事务中获取连接
            //尝试获取当前事务
            currentTransaction = transactionManager.getCurrentTransaction();
            if (currentTransaction != null)
                //如果当前存在事务,则从事务中获取连接
                connection = currentTransaction.getConnection();
            else
                //如果当前不存在事务,则新建一个连接
                connection = transactionManager.newConnection();
        } else {
            //否则使用传入的连接
            connection = this.connection;
        }
        //连接必定不为空
        assert connection != null;

        //替换运行时的数据库名称
        sql = SQLUtils.replaceDatabaseName(sql, databaseNameMapping);
        //执行SQL
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            //使用参数设置器设置参数
            setter.setParameters(statement, parameters);
            //获取返回值
            try (ResultSet rs = statement.executeQuery()) {
                if (rowMapper != null) {
                    //如果指定了row mapper,则使用它来做映射
                    return rowMapper.fetch(rs);
                } else {
                    //如果是单行,则使用BasicTypeMapper
                    if (rs.getMetaData().getColumnCount() == 1) {
                        return new BasicTypeMapper<>(rowType).fetch(rs);
                    } else {
                        return new BeanMapper<>(rowType).fetch(rs);
                    }
                }
            }
        } catch (SQLException e) {
            throw translator.translate(e);
        } finally {
            //从事务管理器中new出来的连接,需要手动关闭
            if (this.transactionManager != null && currentTransaction == null) {
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
