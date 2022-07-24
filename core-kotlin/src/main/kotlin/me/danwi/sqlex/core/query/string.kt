package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.FunctionCallExpression
import me.danwi.sqlex.core.query.expression.ParameterExpression

fun concat(args: Array<Expression>): FunctionCallExpression = Expression.concat(*args)

fun concat(args: Array<String>): FunctionCallExpression =
    Expression.concat(*args.map { it.arg }.toTypedArray())

fun concatWs(separator: String, vararg arg: String): FunctionCallExpression =
    Expression.concatWs(separator.lit, *arg.map { it.arg }.toTypedArray())
