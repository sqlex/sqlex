package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.*
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.core.annotation.entity.SqlExColumnName
import me.danwi.sqlex.parser.Field
import me.danwi.sqlex.parser.util.pascalName
import org.jetbrains.annotations.NotNull
import org.jetbrains.annotations.Nullable
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
            return if (this.unsigned) ClassName.get(BigInteger::class.java) else ClassName.LONG.box()
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
        } else if (this.dbType == "var_string") { //mysql内部类型, 等同于varchar
            return ClassName.get(java.lang.String::class.java)
        } else {
            //return "Object"
            //内测阶段直接抛出异常, 便于排错
            throw Exception("db[${this.dbType}] -> java 映射失败!!!")
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
            throw Exception("db[${this.dbType}] -> jdbc 映射失败!!!")
        }
    }

/**
 * 将字段数组转换成实体类
 */
fun Array<Field>.toEntityClass(
    className: String,
    isStatic: Boolean = false,
    constructor: Boolean = true,
    nullableAnnotation: Boolean = true
): TypeSpec {
    val typeSpecBuilder = TypeSpec.classBuilder(className).addModifiers(Modifier.PUBLIC)
    if (isStatic)
        typeSpecBuilder.addModifiers(Modifier.STATIC)
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
    this.forEach { typeSpecBuilder.addColumnGetterAndSetter(it, nullableAnnotation) }
    //给实体添加构造函数
    if (constructor) {
        //无参构造函数
        typeSpecBuilder.addMethod(MethodSpec.constructorBuilder().addModifiers(Modifier.PUBLIC).build())
        //全字段构造函数
        val allFieldConstructorBuilder = MethodSpec.constructorBuilder()
            .addModifiers(Modifier.PUBLIC)
        this.forEach {
            val fieldName = it.name.pascalName
            val parameterSpec = ParameterSpec.builder(it.JavaType, fieldName)
            if (nullableAnnotation)
                if (it.notNull)
                    parameterSpec.addAnnotation(NotNull::class.java)
                else
                    parameterSpec.addAnnotation(Nullable::class.java)
            allFieldConstructorBuilder.addParameter(parameterSpec.build())
            allFieldConstructorBuilder.addCode("this.$fieldName = $fieldName;\n")
        }
        typeSpecBuilder.addMethod(allFieldConstructorBuilder.build())
        //必要字段的构造函数,必要字段指的是 不能null,且不能自动生成(不自增 && 没有默认值)
        val necessaryColumns = this.filter {
            it.notNull && (!it.isAutoIncrement && !it.hasDefaultValue)
        }
        if (necessaryColumns.isNotEmpty()) {
            val fromMethodBuilder = MethodSpec.methodBuilder("from")
                .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
                .returns(ClassName.bestGuess(className))
                .addCode("$className object = new $className();\n")
            necessaryColumns.forEach {
                val fieldName = it.name.pascalName
                val parameterSpec = ParameterSpec.builder(it.JavaType, fieldName)
                if (nullableAnnotation)
                    parameterSpec.addAnnotation(NotNull::class.java)
                fromMethodBuilder.addParameter(parameterSpec.build())
                fromMethodBuilder.addCode("object.$fieldName = $fieldName;\n")
            }
            fromMethodBuilder.addCode("return object;")
            typeSpecBuilder.addMethod(fromMethodBuilder.build())
        }
    }
    //给实体添加toString
    typeSpecBuilder.addMethod(
        MethodSpec.methodBuilder("toString")
            .addAnnotation(Override::class.java)
            .addModifiers(Modifier.PUBLIC)
            .returns(java.lang.String::class.java)
            .addCode("return \"$className{ \" +")
            .addCode(this.map { it.name.pascalName }.joinToString("+ \", \" +") { "\"$it=\" + $it" })
            .addCode("+ \" }\";").build()
    )
    //给实体添加hashCode
    typeSpecBuilder.addMethod(
        MethodSpec.methodBuilder("hashCode")
            .addAnnotation(Override::class.java)
            .addModifiers(Modifier.PUBLIC)
            .returns(ClassName.INT)
            .addCode("return java.util.Objects.hash(${this.joinToString(", ") { it.name.pascalName }});")
            .build()
    )
    //给实体添加equals方法
    typeSpecBuilder.addMethod(
        MethodSpec.methodBuilder("equals")
            .addAnnotation(Override::class.java)
            .addModifiers(Modifier.PUBLIC)
            .returns(ClassName.BOOLEAN)
            .addParameter(java.lang.Object::class.java, "o")
            .addCode("if (this == o) return true;\n")
            .addCode("if (o == null || getClass() != o.getClass()) return false;\n")
            .addCode("$className other = ($className) o;\n")
            .addCode("return ${
                this
                    .map { it.name.pascalName }
                    .joinToString(" && ")
                    { "java.util.Objects.equals($it, other.$it)" }
            };")
            .build()
    )

    return typeSpecBuilder.build()
}

/**
 * 给实体类添加数据列getter/setter
 */
fun TypeSpec.Builder.addColumnGetterAndSetter(field: Field, nullableAnnotation: Boolean = true): TypeSpec.Builder {
    val columnName = field.name
    val type = field.JavaType
    val notNull = field.notNull
    val pascalName = columnName.pascalName
    this.addField(type, pascalName, Modifier.PRIVATE)

    //getter
    val getterMethodSpec = MethodSpec.methodBuilder("get$pascalName")
        .addAnnotation(
            AnnotationSpec.builder(SqlExColumnName::class.java).addMember("value", "\$S", columnName).build()
        )
        .addModifiers(Modifier.PUBLIC)
        .addStatement("return this.$pascalName")
        .returns(type)
    if (nullableAnnotation)
        if (notNull)
            getterMethodSpec.addAnnotation(NotNull::class.java)
        else
            getterMethodSpec.addAnnotation(Nullable::class.java)
    this.addMethod(getterMethodSpec.build())

    //setter
    val parameterSpec = ParameterSpec.builder(type, "value")
    if (nullableAnnotation)
        if (notNull)
            parameterSpec.addAnnotation(NotNull::class.java)
        else
            parameterSpec.addAnnotation(Nullable::class.java)
    this.addMethod(
        MethodSpec.methodBuilder("set$pascalName")
            .addAnnotation(
                AnnotationSpec.builder(SqlExColumnName::class.java).addMember("value", "\$S", columnName).build()
            )
            .addModifiers(Modifier.PUBLIC)
            .addParameter(parameterSpec.build())
            .addStatement("this.$pascalName = value")
            .build()
    )
    return this
}