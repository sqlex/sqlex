package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.ArrayTypeName
import com.squareup.javapoet.ClassName
import com.squareup.javapoet.TypeName
import me.danwi.sqlex.parser.Field
import java.sql.JDBCType

/**
 * 获取字段对应的Java类型
 */
val Field.JavaType: TypeName
    inline get() {
        if (this.dbType == "bit") { //bit(n)
            return if (this.length == 1L) ClassName.bestGuess("Boolean") else ArrayTypeName.of(ClassName.BYTE)
        } else if (this.dbType == "tinyint") { //tinyint(n) 或者 bool, boolean
            //TODO: tinyInt1isBit为false时, Integer; 为true时, Boolean且size是1. 默认为false
            return ClassName.bestGuess("Integer")
        } else if (listOf("smallint", "mediumint").contains(this.dbType)) { //smallint, mediumint(不管是否unsigned)
            return ClassName.bestGuess("Integer")
        } else if (listOf("int", "integer").contains(this.dbType)) { //int, integer(unsigned时, java.lang.Long)
            return if (this.unsigned) ClassName.bestGuess("Long") else ClassName.bestGuess("Integer")
        } else if (this.dbType == "bigint") { //bigint(unsigned时, java.math.BigInteger)
            return if (this.unsigned) ClassName.bestGuess("Long") else ClassName.bestGuess("java.math.BigInteger")
        } else if (this.dbType == "float") { //float
            return ClassName.bestGuess("Float")
        } else if (this.dbType == "double") { //double
            return ClassName.bestGuess("Double")
        } else if (this.dbType == "decimal") { //decimal
            return ClassName.get(java.math.BigDecimal::class.java)
        } else if (this.dbType == "date") { //date
            return ClassName.get(java.time.LocalDate::class.java)
        } else if (this.dbType == "datetime") { //datetime
            return ClassName.get(java.time.LocalDateTime::class.java)
        } else if (this.dbType == "timestamp") { //timestamp
            return ClassName.get(java.time.OffsetDateTime::class.java)
        } else if (this.dbType == "time") { //time
            return ClassName.get(java.time.LocalTime::class.java)
        } else if (this.dbType == "year") { //year
            return ClassName.get(java.time.LocalDate::class.java)
        } else if (listOf("char", "varchar").contains(this.dbType)) { //char, varchar
            return if (this.binary) ArrayTypeName.of(ClassName.BYTE) else ClassName.bestGuess("String")
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
            return ClassName.bestGuess("String")
        } else {
            //return "Object"
            //内测阶段直接抛出异常, 便于排错
            throw Exception("${this.dbType} 映射失败!!!")
        }
    }

/**
 * 获取字段对应的JDBC类型
 */
val Field.JDBCType: JDBCType
    inline get() = when (this.dbType) {
        "bit" -> java.sql.JDBCType.BIT
        "tinyint" -> java.sql.JDBCType.TINYINT
        "smallint" -> java.sql.JDBCType.SMALLINT
        "mediumint" -> java.sql.JDBCType.INTEGER
        "int" -> java.sql.JDBCType.INTEGER
        "bigint" -> java.sql.JDBCType.BIGINT
        "float" -> java.sql.JDBCType.REAL
        "double" -> java.sql.JDBCType.DOUBLE
        "decimal" -> java.sql.JDBCType.DECIMAL
        "date" -> java.sql.JDBCType.DATE
        "datetime" -> java.sql.JDBCType.TIMESTAMP
        "timestamp" -> java.sql.JDBCType.TIMESTAMP
        "time" -> java.sql.JDBCType.TIME
        "year" -> java.sql.JDBCType.DATE
        "char" -> if (this.binary) java.sql.JDBCType.BINARY else java.sql.JDBCType.CHAR
        "varchar" -> if (this.binary) java.sql.JDBCType.VARBINARY else java.sql.JDBCType.VARCHAR
        "binary" -> java.sql.JDBCType.BINARY
        "varbinary" -> java.sql.JDBCType.VARBINARY
        "tinyblob" -> java.sql.JDBCType.VARBINARY
        "tinytext" -> java.sql.JDBCType.VARCHAR
        "blob" -> java.sql.JDBCType.LONGVARBINARY
        "text" -> java.sql.JDBCType.LONGVARCHAR
        "mediumblob" -> java.sql.JDBCType.LONGVARBINARY
        "mediumtext" -> java.sql.JDBCType.LONGVARCHAR
        "longblob" -> java.sql.JDBCType.LONGVARBINARY
        "longtext" -> java.sql.JDBCType.LONGVARCHAR
        "json" -> java.sql.JDBCType.LONGVARCHAR
        "geometry" -> java.sql.JDBCType.BINARY
        "enum" -> java.sql.JDBCType.CHAR
        "set" -> java.sql.JDBCType.CHAR
        "null" -> java.sql.JDBCType.NULL
        else -> {
            //JDBCType.VARCHAR
            //内测阶段直接抛出异常, 便于排错
            throw Exception("${this.dbType} 映射失败!!!")
        }
    }