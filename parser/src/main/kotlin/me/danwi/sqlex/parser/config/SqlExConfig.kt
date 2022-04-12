package me.danwi.sqlex.parser.config

import me.danwi.sqlex.parser.exception.SqlExParserException
import org.yaml.snakeyaml.TypeDescription
import org.yaml.snakeyaml.Yaml
import org.yaml.snakeyaml.constructor.Constructor

fun createSqlExConfig(content: String): SqlExConfig {
    val constructor = Constructor()
    val configDesc = TypeDescription(SqlExConfig::class.java)
    configDesc.substituteProperty(
        "package",
        String::class.java,
        "getRootPackage",
        "setRootPackage"
    )
    configDesc.substituteProperty(
        "converters",
        List::class.java,
        "getConverters",
        "setConverters"
    )

    constructor.addTypeDescription(configDesc)
    val yaml = Yaml(constructor)
    val config = yaml.loadAs(content, SqlExConfig::class.java)
    config.validate()
    return config
}

class SqlExConfig {
    var rootPackage: String? = null
    var converters: List<String> = listOf()
}

fun SqlExConfig.validate() {
    if (this.rootPackage?.isEmpty() != false)
        throw SqlExParserException("package property is required")
}