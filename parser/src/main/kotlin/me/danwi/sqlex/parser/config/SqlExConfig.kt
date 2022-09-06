package me.danwi.sqlex.parser.config

import com.fasterxml.jackson.annotation.JsonProperty
import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.dataformat.yaml.YAMLFactory
import com.fasterxml.jackson.module.kotlin.registerKotlinModule

data class SqlExConfig(
    @JsonProperty("package")
    val rootPackage: String,
    val converters: List<String> = listOf(),
    val foreign: Map<String, String> = mapOf()
)

private val mapper = ObjectMapper(YAMLFactory()).registerKotlinModule()

fun parseSqlExConfig(content: String): SqlExConfig {
    return mapper.readValue(content, SqlExConfig::class.java)
}