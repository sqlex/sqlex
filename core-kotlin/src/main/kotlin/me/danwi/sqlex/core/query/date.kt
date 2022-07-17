package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression

fun now() = Expression.now()

fun currentTimestamp() = Expression.currentTimestamp()

fun dateFormat(date: Expression, format: Expression) = Expression.dateFormat(date, format)

fun dateFormat(date: Expression, format: String) = Expression.dateFormat(date, format)