package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.*
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.core.annotation.SqlExColumnName
import me.danwi.sqlex.parser.Field
import me.danwi.sqlex.parser.util.pascalName
import java.math.BigDecimal
import java.math.BigInteger
import java.sql.JDBCType
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.OffsetDateTime
import javax.lang.model.element.Modifier

/**
 * 获取字段对应的Java类型
 */
val Field.JavaType: TypeName
    inline get() {
        if (this.dbType == "bit") { //bit(n)
            return if (this.length == 1L) ClassName.BOOLEAN.box() else ArrayTypeName.of(ClassName.BYTE)
        } else if (this.dbType == "tinyint") { //tinyint(n) 或者 bool, boolean
            //TODO: tinyInt1isBit为false时, Integer; 为true时, Boolean且size是1. 默认为false
            return ClassName.INT.box()
        } else if (listOf("smallint", "mediumint").contains(this.dbType)) { //smallint, mediumint(不管是否unsigned)
            return ClassName.INT.box()
        } else if (listOf("int", "integer").contains(this.dbType)) { //int, integer(unsigned时, java.lang.Long)
            return if (this.unsigned) ClassName.LONG.box() else ClassName.INT.box()
        } else if (this.dbType == "bigint") { //bigint(unsigned时, java.math.BigInteger)
            return if (this.unsigned) ClassName.LONG.box() else ClassName.get(BigInteger::class.java)
        } else if (this.dbType == "float") { //float
            return ClassName.FLOAT.box()
        } else if (this.dbType == "double") { //double
            return ClassName.DOUBLE.box()
        } else if (this.dbType == "decimal") { //decimal
            return ClassName.get(BigDecimal::class.java)
        } else if (this.dbType == "date") { //date
            return ClassName.get(LocalDate::class.java)
        } else if (this.dbType == "datetime") { //datetime
            return ClassName.get(LocalDateTime::class.java)
        } else if (this.dbType == "timestamp") { //timestamp
            return ClassName.get(OffsetDateTime::class.java)
        } else if (this.dbType == "time") { //time
            return ClassName.get(LocalTime::class.java)
        } else if (this.dbType == "year") { //year
            return ClassName.get(LocalDate::class.java)
        } else if (listOf("char", "varchar").contains(this.dbType)) { //char, varchar
            return if (this.binary) ArrayTypeName.of(ClassName.BYTE) else ClassName.get(java.lang.String::class.java)
        } else if (listOf(
                "binary",
                "varbinary",
                "tinyblob",
                "blob",
                "mediumblob",
                "longblob"
            ).contains(this.dbType)
        ) { //binary, varbinary, tinyblob, blob, mediumblob, longblob
            return ArrayTypeName.of(ClassName.BYTE)
        } else if (listOf(
                "tinytext",
                "text",
                "mediumtext",
                "longtext",
                "enum",
                "set"
            ).contains(this.dbType)
        ) { //tinytext, text, mediumtext, longtext
            return ClassName.get(java.lang.String::class.java)
        } else {
            //return "Object"
            //内测阶段直接抛出异常, 便于排错
            throw Exception("${this.dbType} 映射失败!!!")
        }
    }

/**
 * 获取字段对应的JDBC类型
 */
val Field.JdbcType: JDBCType
    inline get() = when (this.dbType) {
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
        "char" -> if (this.binary) JDBCType.BINARY else JDBCType.CHAR
        "varchar" -> if (this.binary) JDBCType.VARBINARY else JDBCType.VARCHAR
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
            throw Exception("${this.dbType} 映射失败!!!")
        }
    }

/**
 * 将字段数组转换成实体类
 */
fun Array<Field>.toEntityClass(className: String): TypeSpec {
    val typeSpecBuilder = TypeSpec.classBuilder(className)
        .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
    //判断列名是否重复
    val duplicateFieldNames =
        this.groupBy(keySelector = { it.name.pascalName }, valueTransform = { it.name })
            .filter { it.value.size > 1 }.values
    if (duplicateFieldNames.isNotEmpty()) {
        throw Exception("重复的列名 ${duplicateFieldNames.joinToString(", ")}")
    }
    //判断列名是否非法
    val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
    val invalidFieldNames = this.filter { !regex.matches(it.name.pascalName) }
    if (invalidFieldNames.isNotEmpty()) {
        throw Exception("非法的列名 ${invalidFieldNames.joinToString(", ") { it.name }}")
    }
    //给实体添加getter/setter
    this
        .map { Pair(it.name, it.JavaType) }
        .forEach { typeSpecBuilder.addColumnGetterAndSetter(it.first, it.second) }
    return typeSpecBuilder.build()
}

/**
 * 给实体类添加数据列getter/setter
 */
fun TypeSpec.Builder.addColumnGetterAndSetter(columnName: String, type: TypeName): TypeSpec.Builder {
    this.addField(type, "_${columnName.pascalName}", Modifier.PRIVATE)
    this.addMethod(
        MethodSpec.methodBuilder("get${columnName.pascalName}")
            .addModifiers(Modifier.PUBLIC)
            .addStatement("return this._${columnName.pascalName}")
            .returns(type)
            .build()
    )
    this.addMethod(
        MethodSpec.methodBuilder("set${columnName.pascalName}")
            .addAnnotation(
                AnnotationSpec.builder(SqlExColumnName::class.java).addMember("value", "\$S", columnName).build()
            )
            .addModifiers(Modifier.PUBLIC)
            .addParameter(type, "value")
            .addStatement("this._${columnName.pascalName} = value")
            .build()
    )
    return this
}