package me.danwi.sqlex.core.query

import me.danwi.sqlex.core.query.expression.Expression

fun <T> WhereBuilder<T>.filter(builder: () -> Expression?): WhereBuilder<T>? = this.where(builder.invoke())
