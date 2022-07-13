package me.danwi.sqlex.core.query;

import me.danwi.sqlex.core.query.expression.Expression;
import me.danwi.sqlex.core.type.PagedResult;

import java.util.List;

public class TableQuery<T> extends WhereBuilder<TableQuery<T>> {
    private boolean forUpdate = false;
    private Long skip;
    private Long take;

    /**
     * 排序,默认升序
     *
     * @param exp 排序表达式
     * @return this
     */
    TableQuery<T> order(Expression exp) {
        return order(exp, Order.Asc);
    }

    /**
     * 按照指定顺序排序
     *
     * @param exp   排序表达式
     * @param order 顺序
     * @return this
     */
    TableQuery<T> order(Expression exp, Order order) {
        throw new UnsupportedOperationException();
    }

    /**
     * 跳过多少条记录
     *
     * @param number 跳过的记录数
     * @return this
     */
    TableQuery<T> skip(long number) {
        skip = number;
        return this;
    }

    /**
     * 取多少条记录
     *
     * @param number 记录数量
     * @return this
     */
    TableQuery<T> take(long number) {
        take = number;
        return this;
    }

    /**
     * forUpdate加锁
     *
     * @return this
     */
    TableQuery<T> forUpdate() {
        forUpdate = true;
        return this;
    }

    /**
     * 统计行数
     *
     * @return 行数
     */
    long count() {
        throw new UnsupportedOperationException();
    }

    /**
     * 执行请求并获取到结果
     *
     * @return 结果
     */
    List<T> find() {
        throw new UnsupportedOperationException();
    }

    /**
     * 执行请求,获取到第一条数据
     *
     * @return 第一条数据
     */
    T findOne() {
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
    PagedResult<T> page(long pageSize, long pageNo) {
        long total = this.count();
        List<T> data = this.find();
        return new PagedResult<>(pageSize, pageNo, total, data);
    }
}

