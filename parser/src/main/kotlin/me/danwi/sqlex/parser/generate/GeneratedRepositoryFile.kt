package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.core.RepositoryLike
import me.danwi.sqlex.core.annotation.SqlExConverter
import me.danwi.sqlex.core.annotation.SqlExConverterCheck
import me.danwi.sqlex.core.annotation.SqlExSchema
import me.danwi.sqlex.core.annotation.SqlExTableInfo
import me.danwi.sqlex.parser.Session
import javax.lang.model.element.Modifier

const val RepositoryClassName = "Repository"

class GeneratedRepositoryFile(
    rootPackage: String,
    private val converters: List<String>,
    private val schemas: List<String>,
    private val session: Session,
) : GeneratedJavaFile(rootPackage, RepositoryClassName) {
    override fun generate(): TypeSpec {
        return TypeSpec.interfaceBuilder(RepositoryClassName)
            .addModifiers(Modifier.PUBLIC)
            .addSuperinterface(RepositoryLike::class.java)
            .addAnnotation(
                AnnotationSpec.builder(SqlExConverterCheck::class.java)
                    .build()
            )
            .addAnnotations(
                converters.mapIndexed { order, converter ->
                    AnnotationSpec
                        .builder(SqlExConverter::class.java)
                        .addMember("order", "\$L", order)
                        .addMember("converter", "\$L.class", converter)
                        .build()
                }
            )
            .addAnnotations(
                schemas.mapIndexed { version, script ->
                    //将script解析为单个语句
                    val sqls = session.getSQLsOfScript(script)
                    val builder = AnnotationSpec
                        .builder(SqlExSchema::class.java)
                        .addMember("version", "\$L", version)
                    sqls.forEach { builder.addMember("scripts", "\$S", it) }
                    builder.build()
                }
            )
            .addAnnotations(
                session.allTables.map { table ->
                    val builder = AnnotationSpec
                        .builder(SqlExTableInfo::class.java)
                        .addMember("name", "\$S", table)
                    session.getFields("select * from $table").forEach {
                        builder.addMember("columnNames", "\$S", it.name)
                            .addMember("columnTypeIds", "\$S", getJavaType(it.dbType, it.unsigned))
                            .addMember("columnTypeNames", "\$S", it.dbType)
                            .addMember("columnLengths", "\$S", it.length)
                            .addMember("columnUnsigneds", "\$S", it.unsigned)
                    }
                    builder.build()
                }
            )
            .build()
    }

    private fun getJavaType(mysqlType: String, isBinary: Boolean): Int {
        return when (mysqlType) {
            "tinyint" -> -7
            "smallint" -> 5
            "int" -> 4
            "float" -> 7
            "double" -> 8
            "null" -> 0
            "decimal" -> 3
            "timestamp", "datetime" -> 93
            "bigint" -> -5
            "mediumint" -> 4
            // mysql.TypeNewDate
            "date" -> 91
            "time" -> 92
            "year" -> 12
            "enum" -> 4
            "set" -> -7
            "tinytext" -> -3
            "mediumtext", "longtext", "text" -> -4
            "var_string", "varchar" -> if (isBinary) -3 else 12
            "char" -> if (isBinary) -2 else 1
            "geometry" -> -2
            "bit" -> -7
            "json" -> 12
            else -> 12
        }
    }
}