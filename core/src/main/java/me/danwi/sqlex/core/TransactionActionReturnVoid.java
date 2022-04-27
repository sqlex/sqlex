package me.danwi.sqlex.core;

import me.danwi.sqlex.core.transaction.Transaction;

/**
 * 包裹在事务中运行的函数,没有返回值
 */
@FunctionalInterface
public interface TransactionActionReturnVoid {
    void run(Transaction transaction) throws Exception;
}
