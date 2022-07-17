package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.BinaryExpression
import me.danwi.sqlex.core.query.expression.Expression

operator fun Expression.plus(right: Expression) = BinaryExpression("+", this, right)
operator fun Expression.plus(right: Any) = BinaryExpression("+", this, Expression.arg(right))

operator fun Expression.minus(right: Expression) = BinaryExpression("-", this, right)
operator fun Expression.minus(right: Any) = BinaryExpression("-", this, Expression.arg(right))

operator fun Expression.times(right: Expression) = BinaryExpression("*", this, right)
operator fun Expression.times(right: Any) = BinaryExpression("*", this, Expression.arg(right))

operator fun Expression.div(right: Expression) = BinaryExpression("/", this, right)
operator fun Expression.div(right: Any) = BinaryExpression("/", this, Expression.arg(right))
