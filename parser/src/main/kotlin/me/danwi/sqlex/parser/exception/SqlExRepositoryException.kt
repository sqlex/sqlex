package me.danwi.sqlex.parser.exception

open class SqlExRepositoryException(message: String) : SqlExParserException(message) {

}

class SqlExRepositorySchemaException(val relativePath: String, message: String?) :
    SqlExRepositoryException(message ?: "未知的Schema解析错误") {

}

class SqlExRepositoryMethodException(val relativePath: String, message: String) : SqlExRepositoryException(message) {

}

class SqlExRepositoryGenerateException(val filePath: String, message: String) : SqlExRepositoryException(message) {}