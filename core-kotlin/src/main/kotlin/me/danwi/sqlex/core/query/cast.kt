package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.CastExpression
import me.danwi.sqlex.core.query.expression.Expression

fun cast(exp: Expression, type: CastExpression.Type): CastExpression = Expression.cast(exp, type)

fun cast(exp: Expression, type: CastExpression.Type, length: Long): CastExpression = Expression.cast(exp, type, length)

fun cast(exp: Expression, type: CastExpression.Type, precision: Long, scale: Long): CastExpression =
    Expression.cast(exp, type, precision, scale)

fun Expression.toBinary(length: Long? = null) =
    if (length == null) cast(this, CastExpression.Type.BINARY) else cast(this, CastExpression.Type.BINARY, length)

fun Expression.toChar(length: Long? = null) =
    if (length == null) cast(this, CastExpression.Type.CHAR) else cast(this, CastExpression.Type.CHAR, length)

fun Expression.toDate() = cast(this, CastExpression.Type.DATE)

fun Expression.toDateTime(secondsPrecision: Long? = null) =
    if (secondsPrecision == null)
        cast(this, CastExpression.Type.DATETIME)
    else
        cast(this, CastExpression.Type.DATETIME, secondsPrecision)

fun Expression.toDecimal(precision: Long? = null, scale: Long? = null) =
    if (precision == null || scale == null)
        cast(this, CastExpression.Type.DECIMAL)
    else
        cast(this, CastExpression.Type.DECIMAL, precision, scale)

fun Expression.toDouble() = cast(this, CastExpression.Type.DOUBLE)

fun Expression.toFloat(precision: Long? = null) =
    if (precision == null) cast(this, CastExpression.Type.FLOAT) else cast(this, CastExpression.Type.FLOAT, precision)

fun Expression.toJson() =
    cast(this, CastExpression.Type.JSON)

fun Expression.toNChar(length: Long? = null) =
    if (length == null) cast(this, CastExpression.Type.NCHAR) else cast(this, CastExpression.Type.NCHAR, length)

fun Expression.toReal() = cast(this, CastExpression.Type.REAL)

fun Expression.toSigned() = cast(this, CastExpression.Type.SIGNED)

fun Expression.toTime(secondsPrecision: Long? = null) =
    if (secondsPrecision == null)
        cast(this, CastExpression.Type.TIME)
    else
        cast(this, CastExpression.Type.TIME, secondsPrecision)

fun Expression.toUnsigned() = cast(this, CastExpression.Type.UNSIGNED)

fun Expression.toYear() = cast(this, CastExpression.Type.YEAR)