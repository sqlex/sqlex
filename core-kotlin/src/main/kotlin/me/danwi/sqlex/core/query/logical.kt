package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.BinaryExpression
import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.UnaryExpression

infix fun Expression.and(right: Expression): BinaryExpression = this.and(right)
infix fun Expression.and(right: Any): BinaryExpression = this.add(right.arg)

infix fun Expression.or(right: Expression): BinaryExpression = this.or(right)
infix fun Expression.or(right: Any): BinaryExpression = this.or(right.arg)

operator fun Expression.not(): UnaryExpression = Expression.not(this)
