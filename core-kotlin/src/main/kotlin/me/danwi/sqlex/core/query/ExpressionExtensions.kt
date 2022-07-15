package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.BinaryExpression
import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.UnaryExpression

infix fun Expression.and(right: Expression) = BinaryExpression("and", this, right)
infix fun Expression.and(right: Any) = BinaryExpression("and", this, Expression.arg(right))

infix fun Expression.or(right: Expression) = BinaryExpression("or", this, right)
infix fun Expression.or(right: Any) = BinaryExpression("or", this, Expression.arg(right))

operator fun Expression.not() = UnaryExpression("!", this)

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

operator fun Expression.plus(right: Expression) = BinaryExpression("+", this, right)
operator fun Expression.plus(right: Any) = BinaryExpression("+", this, Expression.arg(right))

operator fun Expression.minus(right: Expression) = BinaryExpression("-", this, right)
operator fun Expression.minus(right: Any) = BinaryExpression("-", this, Expression.arg(right))

operator fun Expression.times(right: Expression) = BinaryExpression("*", this, right)
operator fun Expression.times(right: Any) = BinaryExpression("*", this, Expression.arg(right))

operator fun Expression.div(right: Expression) = BinaryExpression("/", this, right)
operator fun Expression.div(right: Any) = BinaryExpression("/", this, Expression.arg(right))

infix fun Expression.like(right: Expression) = BinaryExpression("like", this, right)
infix fun Expression.like(right: String) = BinaryExpression("like", this, Expression.arg(right))