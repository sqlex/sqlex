package me.danwi.sqlex.parser.util

import me.danwi.sqlex.parser.exception.SqlExParserException
import org.apache.commons.text.StringEscapeUtils
import java.io.File

//SqlEx生成目录下的标记文件
const val SqlExGeneratedTagFileName = "generate-by-sqlex.txt"

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

//计算相对路径
fun String.relativePathTo(parentPath: String): String {
    return this.windowsPathNormalize.removePrefix(parentPath.windowsPathNormalize).removePrefix("/")
}

//java包名和相对路径的转换
val String.relativePathToPackageName: String
    inline get() {
        val normalizedPath = this.windowsPathNormalize
        //有文件名
        return if (normalizedPath.substringAfterLast("/").contains(".")) {
            normalizedPath
                .substringBeforeLast('/')
                .removePrefix("/").removeSuffix("/")
                .replace('/', '.')
        } else {
            normalizedPath
                .removePrefix("/").removeSuffix("/")
                .replace('/', '.')
        }
    }

val String.packageNameToRelativePath: String
    inline get() = this.windowsPathNormalize.replace('.', '/')

//sqlm路径到java路径到转换
val String.sqlmPathToJavaPath: String
    inline get() = this.windowsPathNormalize.removeSuffix(".$SqlExMethodExtensionName") + ".java"

val String.classNameOfJavaRelativePath: String
    inline get() = this.windowsPathNormalize
        .substringAfterLast('/')
        .removePrefix("/")
        .removeSuffix(".java")

//获取schema文件的版本号
val String.schemaFileVersion: Int?
    inline get() = Regex("^(\\d+)").find(this.windowsPathNormalize.substringAfterLast("/"))?.groups?.get(0)?.value?.toInt()

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
                    throw SqlExParserException("不能使用非命名参数(?)")
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