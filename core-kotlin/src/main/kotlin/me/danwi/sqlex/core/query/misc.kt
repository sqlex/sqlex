package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.FunctionCallExpression
import me.danwi.sqlex.core.query.expression.ParameterExpression

fun func(name: String, vararg args: Expression) = FunctionCallExpression(name, args.asList())

fun arg(value: Any?) = ParameterExpression(value)