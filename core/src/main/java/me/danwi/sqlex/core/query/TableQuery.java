package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.jdbc.RawSQLExecutor;
import me.danwi.sqlex.core.jdbc.mapper.BasicTypeMapper;
import me.danwi.sqlex.core.jdbc.mapper.BeanMapper;
import me.danwi.sqlex.core.query.expression.Expression;
import me.danwi.sqlex.core.query.expression.ExpressionUtil;
import me.danwi.sqlex.core.type.PagedResult;
import org.jetbrains.annotations.Nullable;

import java.util.ArrayList;
import java.util.LinkedList;
import java.util.List;

public class TableQuery<T> extends WhereBuilder<TableQuery<T>> {
    private final String tableName;
    private final RawSQLExecutor executor;
    private final BeanMapper<T> beanMapper;
    private final List<OrderPair> orders = new ArrayList<>();
    private Long skip;
    private Long take;
    private boolean forUpdate = false;

    private static class OrderPair {
        Expression expression;
        Order order;
    }

    public TableQuery(String tableName, RawSQLExecutor executor, Class<T> entityClass) {
        this.tableName = tableName;
        this.executor = executor;
        this.beanMapper = new BeanMapper<>(entityClass);
    }

    /**
     * 排序,默认升序
     *
     * @param exp 排序表达式
     * @return this
     */
    public TableQuery<T> order(Expression exp) {
        return order(exp, Order.Asc);
    }

    /**
     * 按照指定顺序排序
     *
     * @param exp   排序表达式
     * @param order 顺序
     * @return this
     */
    public TableQuery<T> order(Expression exp, Order order) {
        OrderPair pair = new OrderPair();
        pair.expression = exp;
        pair.order = order;
        orders.add(pair);
        return this;
    }

    /**
     * 跳过多少条记录
     *
     * @param number 跳过的记录数
     * @return this
     */
    public TableQuery<T> skip(long number) {
        skip = number;
        return this;
    }

    /**
     * 取多少条记录
     *
     * @param number 记录数量
     * @return this
     */
    public TableQuery<T> take(long number) {
        take = number;
        return this;
    }

    /**
     * forUpdate加锁
     *
     * @return this
     */
    public TableQuery<T> forUpdate() {
        forUpdate = true;
        return this;
    }

    //构建SQL
    private SQLParameterBind buildSQL() {
        String sql = "select * from `" + tableName + "`";
        List<Object> parameters = new LinkedList<>();
        //处理where条件
        if (this.whereCondition != null) {
            SQLParameterBind sqlParameterBind = ExpressionUtil.toSQL(this.whereCondition);
            sql = sql + " where " + sqlParameterBind.getSQL();
            parameters.addAll(sqlParameterBind.getParameters());
        }
        //处理order
        if (!orders.isEmpty()) {
            sql = sql + " order by ";
            List<String> orderSegments = new LinkedList<>();
            for (OrderPair order : orders) {
                SQLParameterBind sqlParameterBind = ExpressionUtil.toSQL(order.expression);
                orderSegments.add("(" + sqlParameterBind.getSQL() + ") " + (order.order == Order.Asc ? "asc" : "desc"));
                parameters.addAll(sqlParameterBind.getParameters());
            }
            sql = sql + String.join(", ", orderSegments);
        }
        //处理limit相关
        if (this.skip != null && this.take != null)
            sql = sql + String.format(" limit %d, %d", this.skip, this.take);
        else if (this.skip != null) {
            sql = sql + String.format(" limit %d, 18446744073709551615", this.skip);
        } else if (this.take != null) {
            sql = sql + " limit " + this.take;
        }
        //处理for update
        if (this.forUpdate)
            sql += " for update";
        return new SQLParameterBind(sql, parameters);
    }

    /**
     * 统计行数
     *
     * @return 行数
     */
    public long count() {
        //构建SQL
        SQLParameterBind sqlParameterBind = this.buildSQL();
        //添加count
        String countSQL = "select count(1) from (" + sqlParameterBind.getSQL() + ") temp";
        //执行
        return executor.query(new BasicTypeMapper<>(Long.class), null, countSQL, sqlParameterBind.getParameters()).get(0);
    }

    /**
     * 执行请求并获取到结果
     *
     * @return 结果
     */
    public List<T> find() {
        //构建SQL
        SQLParameterBind sqlParameterBind = this.buildSQL();
        //执行
        return executor.query(this.beanMapper, null, sqlParameterBind.getSQL(), sqlParameterBind.getParameters());
    }

    /**
     * 执行请求,获取到第一条数据
     *
     * @return 第一条数据
     */
    @Nullable
    public T findOne() {
        List<T> results = this.take(1).find();
        if (results.size() > 0)
            return results.get(0);
        return null;
    }

    /**
     * 分页方式执行请求
     *
     * @param pageSize 分页大小
     * @param pageNo   页码
     * @return 分页结果
     */
    public PagedResult<T> page(long pageSize, long pageNo) {
        long total = this.count();
        this.skip(pageSize * (pageNo - 1)).take(pageSize);
        List<T> data = this.find();
        return new PagedResult<>(pageSize, pageNo, total, data);
    }
}

