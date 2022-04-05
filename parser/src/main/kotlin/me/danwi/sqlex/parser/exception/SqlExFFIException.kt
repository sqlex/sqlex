package me.danwi.sqlex.parser.exception

open class SqlExFFIException(message: String) : SqlExParserException(message) {
}

class SqlExFFIInvokeException(val module: String, val method: String, message: String) : SqlExFFIException(message) {
}