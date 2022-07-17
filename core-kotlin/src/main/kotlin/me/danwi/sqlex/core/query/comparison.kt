package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.BinaryExpression
import me.danwi.sqlex.core.query.expression.Expression

infix fun Expression.eq(right: Expression): BinaryExpression = this.eq(right)
infix fun Expression.eq(right: Any): BinaryExpression = this.eq(right.arg)

infix fun Expression.ne(right: Expression): BinaryExpression = this.ne(right)
infix fun Expression.ne(right: Any): BinaryExpression = this.ne(right.arg)

infix fun Expression.gt(right: Expression): BinaryExpression = this.gt(right)
infix fun Expression.gt(right: Any): BinaryExpression = this.gt(right.arg)

infix fun Expression.gte(right: Expression): BinaryExpression = this.gte(right)
infix fun Expression.gte(right: Any): BinaryExpression = this.gte(right.arg)

infix fun Expression.lt(right: Expression): BinaryExpression = this.lt(right)
infix fun Expression.lt(right: Any): BinaryExpression = this.lt(right.arg)

infix fun Expression.lte(right: Expression): BinaryExpression = this.lte(right)
infix fun Expression.lte(right: Any): BinaryExpression = this.lte(right.arg)

infix fun Expression.like(right: Expression): BinaryExpression = this.like(right)
infix fun Expression.like(right: String): BinaryExpression = this.like(right.arg)