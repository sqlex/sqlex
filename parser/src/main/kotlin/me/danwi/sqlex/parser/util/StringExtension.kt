package me.danwi.sqlex.parser.util

import org.apache.commons.text.StringEscapeUtils
import java.io.File


//SqlEx配置文件名
const val SqlExConfigFileName = "sqlex.yaml"

//SqxEx Schema文件拓展名
const val SqlExSchemaExtensionName = "sqls"

//SqlEx Method文件拓展名
const val SqlExMethodExtensionName = "sqlm"

//字符串是否为一个SqlEx配置文件名
val String?.isSqlExConfigFileName: Boolean
    inline get() = this == SqlExConfigFileName

//字符串是否为一个SqlEx配置文件路径
val String?.isSqlExConfigFilePath: Boolean
    inline get() = this?.windowsPathNormalize?.endsWith("/$SqlExConfigFileName") ?: false

//字符串是否为一个SqlEx Schema文件路径
val String?.isSqlExSchemaFilePath: Boolean
    inline get() = this?.endsWith(".$SqlExSchemaExtensionName") ?: false

//字符串是否为一个SqlEx方法文件路径
val String?.isSqlExMethodFilePath: Boolean
    inline get() = this?.endsWith(".$SqlExMethodExtensionName") ?: false

//windows路径归一化
val String.windowsPathNormalize: String
    inline get() {
        if (System.getProperty("os.name").startsWith("Windows")) {
            return this.replace(File.separatorChar, '/')
        }
        return this
    }

//转化成pascal命名
val String.pascalName: String
    get() {
        if (this.isEmpty()) return ""
        return this.split("_", "-").joinToString("") { it.firstUpperCase }
    }

private val String.firstUpperCase: String
    get() {
        if (this.isEmpty()) return ""
        return this[0].uppercaseChar() + this.substring(1)
    }

//SQL命名参数化
data class NamedParameter(val name: String, val position: Int)
data class NamedParameterSQL(val sql: String, val parameters: List<NamedParameter>)

val String.namedParameterSQL: NamedParameterSQL
    get() {
        val orderedParameters = mutableListOf<Pair<String, Int>>()

        val parsedQuery = StringBuffer(length)
        var inSingleQuote = false
        var inDoubleQuote = false
        var inSingleLineComment = false
        var inMultiLineComment = false
        var inDoubleColon = false

        var index = 0
        while (index < length) {
            var char = this[index]
            if (inSingleQuote) {
                if (char == '\'') {
                    inSingleQuote = false
                }
            } else if (inDoubleQuote) {
                if (char == '"') {
                    inDoubleQuote = false
                }
            } else if (inMultiLineComment) {
                if (char == '*' && this[index + 1] == '/') {
                    inMultiLineComment = false
                }
            } else if (inDoubleColon) {
                if (!Character.isJavaIdentifierPart(char)) {
                    inDoubleColon = false
                }
            } else if (inSingleLineComment) {
                if (char == '\n') {
                    inSingleLineComment = false
                }
            } else {
                if (char == '\'') {
                    inSingleQuote = true
                } else if (char == '"') {
                    inDoubleQuote = true
                } else if (char == '/' && this[index + 1] == '*') {
                    inMultiLineComment = true
                } else if (char == '-' && this[index + 1] == '-') {
                    inSingleLineComment = true
                } else if (char == ':' && this[index + 1] == ':') {
                    inDoubleColon = true
                } else if (char == '?') {
                    throw Exception("不能使用非命名参数(?)")
                } else if (char == ':' && index + 1 < length && Character.isJavaIdentifierStart(this[index + 1])) {
                    var parameterIndex = index + 2
                    while (parameterIndex < length && Character.isJavaIdentifierPart(this[parameterIndex])) {
                        parameterIndex++
                    }
                    val name = this.substring(index + 1, parameterIndex)
                    orderedParameters.add(Pair(name, parsedQuery.length))
                    char = '?'
                    index += name.length
                }
            }
            parsedQuery.append(char)
            index++
        }
        return NamedParameterSQL(parsedQuery.toString(), orderedParameters.map { NamedParameter(it.first, it.second) })
    }

//转义
val String.literal: String
    inline get() = StringEscapeUtils.escapeJava(this)