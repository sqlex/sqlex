package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression

/**
 * 通过 and 将表达式组合起来
 */
fun Iterable<Expression>.joinByAnd(): Expression? = Expression.joinByAnd(this)

/**
 * 通过 or 将表达式组合起来
 */
fun Iterable<Expression>.joinByOr(): Expression? = Expression.joinByOr(this)