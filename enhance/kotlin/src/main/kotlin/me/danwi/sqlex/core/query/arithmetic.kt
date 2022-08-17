package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.BinaryExpression
import me.danwi.sqlex.core.query.expression.Expression

operator fun Expression.plus(right: Expression): BinaryExpression = this.add(right)
operator fun Expression.plus(right: Any): BinaryExpression = this.add(right.arg)

operator fun Expression.minus(right: Expression): BinaryExpression = this.sub(right)
operator fun Expression.minus(right: Any): BinaryExpression = this.sub(right.arg)

operator fun Expression.times(right: Expression): BinaryExpression = this.mul(right)
operator fun Expression.times(right: Any): BinaryExpression = this.mul(right.arg)

operator fun Expression.div(right: Expression): BinaryExpression = this.div(right)
operator fun Expression.div(right: Any): BinaryExpression = this.div(right.arg)
