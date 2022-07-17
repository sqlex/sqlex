package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.FunctionCallExpression

fun now(): FunctionCallExpression {
    return Expression.now()
}

fun currentTimestamp(): FunctionCallExpression {
    return Expression.currentTimestamp()
}

fun dateFormat(date: Expression, format: Expression): FunctionCallExpression {
    return Expression.dateFormat(date, format)
}

fun dateFormat(date: Expression, format: String): FunctionCallExpression {
    return Expression.dateFormat(date, format)
}
