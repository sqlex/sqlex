package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.FunctionCallExpression
import me.danwi.sqlex.core.query.expression.LiteralExpression
import me.danwi.sqlex.core.query.expression.ParameterExpression

fun func(name: String, vararg args: Expression) = FunctionCallExpression(name, args.asList())

fun arg(value: Any?): ParameterExpression = Expression.arg(value)

val Any?.arg: ParameterExpression
    inline get() = Expression.arg(this)

val Any?.lit: LiteralExpression
    inline get() = Expression.lit(this)