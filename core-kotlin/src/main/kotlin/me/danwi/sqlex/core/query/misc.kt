package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.FunctionCallExpression
import me.danwi.sqlex.core.query.expression.LiteralExpression
import me.danwi.sqlex.core.query.expression.ParameterExpression
import me.danwi.sqlex.core.query.expression.RawExpression

fun func(name: String, vararg args: Expression) = FunctionCallExpression(name, args.asList())

fun arg(value: Any?): ParameterExpression = Expression.arg(value)

fun lit(value: Any?): LiteralExpression = Expression.lit(value)

fun sql(raw: String): RawExpression = Expression.sql(raw)

val Any?.arg
    inline get() = arg(this)

val Any?.lit
    inline get() = lit(this)

val String.sql
    inline get() = sql(this)