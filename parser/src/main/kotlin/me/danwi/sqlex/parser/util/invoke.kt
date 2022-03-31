package me.danwi.sqlex.parser.util

//调用方法,收纳异常
fun invokeWithoutException(block: () -> Unit) {
    try {
        block()
    } catch (_: Exception) {

    }
}