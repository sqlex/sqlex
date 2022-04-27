package me.danwi.sqlex.core;

import me.danwi.sqlex.core.transaction.Transaction;

/**
 * 包裹在事务中运行的函数, Also see {@link TransactionActionReturnVoid}
 *
 * @param <T> 返回值类型
 */
@FunctionalInterface
public interface TransactionAction<T> {
    T run(Transaction transaction) throws Exception;
}
