package me.danwi.sqlex.parser.exception

open class SqlExFFIException(message: String) : SqlExParserException(message) {
    override val message: String
        get() = "调用FFI时发生错误: ${super.message ?: "未知错误"}"
}

class SqlExFFIInvokeException(val module: String, val method: String, message: String) : SqlExFFIException(message) {
    override val message: String
        get() = "调用FFI ${module}.${method} 方法时发生错误: ${super.message}"
}