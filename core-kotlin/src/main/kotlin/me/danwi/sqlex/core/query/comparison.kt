package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.BinaryExpression
import me.danwi.sqlex.core.query.expression.Expression

infix fun Expression.eq(right: Expression) = BinaryExpression("=", this, right)
infix fun Expression.eq(right: Any) = BinaryExpression("=", this, Expression.arg(right))

infix fun Expression.ne(right: Expression) = BinaryExpression("<>", this, right)
infix fun Expression.ne(right: Any) = BinaryExpression("<>", this, Expression.arg(right))

infix fun Expression.gt(right: Expression) = BinaryExpression(">", this, right)
infix fun Expression.gt(right: Any) = BinaryExpression(">", this, Expression.arg(right))

infix fun Expression.gte(right: Expression) = BinaryExpression(">=", this, right)
infix fun Expression.gte(right: Any) = BinaryExpression(">=", this, Expression.arg(right))

infix fun Expression.lt(right: Expression) = BinaryExpression("<", this, right)
infix fun Expression.lt(right: Any) = BinaryExpression("<", this, Expression.arg(right))

infix fun Expression.lte(right: Expression) = BinaryExpression("<=", this, right)
infix fun Expression.lte(right: Any) = BinaryExpression("<=", this, Expression.arg(right))

infix fun Expression.like(right: Expression) = BinaryExpression("like", this, right)
infix fun Expression.like(right: String) = BinaryExpression("like", this, Expression.arg(right))