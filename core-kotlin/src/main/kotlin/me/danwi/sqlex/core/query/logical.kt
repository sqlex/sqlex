package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.BinaryExpression
import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.core.query.expression.UnaryExpression

infix fun Expression.and(right: Expression) = BinaryExpression("and", this, right)
infix fun Expression.and(right: Any) = BinaryExpression("and", this, Expression.arg(right))

infix fun Expression.or(right: Expression) = BinaryExpression("or", this, right)
infix fun Expression.or(right: Any) = BinaryExpression("or", this, Expression.arg(right))

operator fun Expression.not() = UnaryExpression("!", this)
