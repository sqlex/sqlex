package me.danwi.sqlex.parser.config

import com.charleskorn.kaml.Yaml
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class SqlExConfig(
    @SerialName("package")
    val rootPackage: String,
    val converters: List<String> = listOf()
)

fun parseSqlExConfig(content: String): SqlExConfig {
    return Yaml.default.decodeFromString(SqlExConfig.serializer(), content)
}