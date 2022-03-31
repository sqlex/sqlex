package me.danwi.sqlex.parser.config

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
    constructor.addTypeDescription(configDesc)
    val yaml = Yaml(constructor)
    val config = yaml.loadAs(content, SqlExConfig::class.java)
    config.validate()
    return config
}

class SqlExConfig {
    var rootPackage: String? = null
}

fun SqlExConfig.validate() {
    if (this.rootPackage?.isEmpty() != false)
        throw Exception("package property is required")
}