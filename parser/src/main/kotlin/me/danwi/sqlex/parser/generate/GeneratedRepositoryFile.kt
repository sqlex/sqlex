package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.core.RepositoryLike
import me.danwi.sqlex.core.annotation.SqlExConverter
import me.danwi.sqlex.core.annotation.SqlExConverterCheck
import me.danwi.sqlex.core.annotation.SqlExSchema
import me.danwi.sqlex.core.annotation.SqlExTableInfo
import me.danwi.sqlex.parser.Field
import me.danwi.sqlex.parser.Session
import java.sql.JDBCType
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
                            .addMember("columnTypeIds", "\$S", getJDBCType(it).vendorTypeNumber)
                            .addMember("columnTypeNames", "\$S", it.dbType)
                            .addMember("columnLengths", "\$S", it.length)
                            .addMember("columnUnsigneds", "\$S", it.unsigned)
                    }
                    builder.build()
                }
            )
            .build()
    }

    private fun getJDBCType(field: Field): JDBCType {
        return when (field.dbType) {
            "bit" -> JDBCType.BIT
            "tinyint" -> JDBCType.TINYINT
            "smallint" -> JDBCType.SMALLINT
            "mediumint" -> JDBCType.INTEGER
            "int" -> JDBCType.INTEGER
            "bigint" -> JDBCType.BIGINT
            "float" -> JDBCType.REAL
            "double" -> JDBCType.DOUBLE
            "decimal" -> JDBCType.DECIMAL
            "date" -> JDBCType.DATE
            "datetime" -> JDBCType.TIMESTAMP
            "timestamp" -> JDBCType.TIMESTAMP
            "time" -> JDBCType.TIME
            "year" -> JDBCType.DATE
            "char" -> if (field.binary) JDBCType.BINARY else JDBCType.CHAR
            "varchar" -> if (field.binary) JDBCType.VARBINARY else JDBCType.VARCHAR
            "binary" -> JDBCType.BINARY
            "varbinary" -> JDBCType.VARBINARY
            "tinyblob" -> JDBCType.VARBINARY
            "tinytext" -> JDBCType.VARCHAR
            "blob" -> JDBCType.LONGVARBINARY
            "text" -> JDBCType.LONGVARCHAR
            "mediumblob" -> JDBCType.LONGVARBINARY
            "mediumtext" -> JDBCType.LONGVARCHAR
            "longblob" -> JDBCType.LONGVARBINARY
            "longtext" -> JDBCType.LONGVARCHAR
            "json" -> JDBCType.LONGVARCHAR
            "geometry" -> JDBCType.BINARY
            "enum" -> JDBCType.CHAR
            "set" -> JDBCType.CHAR
            "null" -> JDBCType.NULL
            else -> {
                //JDBCType.VARCHAR
                //内测阶段直接抛出异常, 便于排错
                throw Exception("${field.dbType} 映射失败!!!")
            }
        }
    }
}