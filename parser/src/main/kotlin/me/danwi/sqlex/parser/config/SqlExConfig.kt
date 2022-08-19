package me.danwi.sqlex.parser.config

import com.charleskorn.kaml.MissingRequiredPropertyException
import com.charleskorn.kaml.UnknownPropertyException
import com.charleskorn.kaml.Yaml
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import me.danwi.sqlex.parser.exception.SqlExParserException

@Serializable
data class SqlExConfig(
    @SerialName("package")
    val rootPackage: String,
    val converters: List<String> = listOf(),
    val foreign: Map<String, String> = mapOf()
)

fun parseSqlExConfig(content: String): SqlExConfig {
    try {
        return Yaml.default.decodeFromString(SqlExConfig.serializer(), content)
    } catch (e: MissingRequiredPropertyException) {
        throw SqlExParserException("配置文件错误, 缺少 ${e.propertyName} 配置, (位置 ${e.line}:${e.column})")
    } catch (e: UnknownPropertyException) {
        throw SqlExParserException("配置文件错误, 未知的配置属性 ${e.propertyName}, (位置 ${e.line}:${e.column})")
    }
}