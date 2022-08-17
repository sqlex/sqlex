package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.FunctionCallExpression

fun now(): FunctionCallExpression = Expression.now()

fun currentTimestamp(): FunctionCallExpression = Expression.currentTimestamp()

fun dateFormat(date: Expression, format: Expression): FunctionCallExpression = Expression.dateFormat(date, format)

fun dateFormat(date: Expression, format: String): FunctionCallExpression = Expression.dateFormat(date, format)