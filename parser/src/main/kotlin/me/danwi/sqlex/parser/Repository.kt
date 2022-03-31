package me.danwi.sqlex.parser

import me.danwi.sqlex.parser.config.SqlExConfig
import me.danwi.sqlex.parser.util.*
import org.antlr.v4.runtime.CharStreams
import org.antlr.v4.runtime.CommonTokenStream
import java.util.concurrent.atomic.AtomicInteger

//数据名称索引
private val index = AtomicInteger()

class RepositorySchemaResolveException(override val message: String?) : Exception(message)

class RepositoryBuilder(private val config: SqlExConfig) {
    private val databaseName = "sqlex_${index.getAndIncrement()}"

    private val session: Session = Session(databaseName)

    fun addSchema(script: String) {
        try {
            session.executeScript(script)
        } catch (e: Exception) {
            try {
                session.execute("drop database $databaseName")
            } finally {
                session.close()
                throw RepositorySchemaResolveException(e.message)
            }
        }
    }

    fun build(): Repository {
        try {
            return Repository(databaseName, Session(databaseName), config)
        } catch (e: Exception) {
            try {
                session.execute("drop database $databaseName")
            } finally {
                throw e
            }
        } finally {
            session.close()
        }
    }

    fun close() {
        try {
            session.execute("drop database $databaseName")
        } catch (_: Exception) {
        } finally {
            session.close()
        }
    }
}

class Repository(private val databaseName: String, private val session: Session, private val config: SqlExConfig) {
    val DDL: String
        get() = session.DDL

    fun generateJavaFile(relativePath: String, content: String): JavaFile {
        //java相对路径名
        val javaRelativePath = relativePath.windowsPathNormalize.removeSuffix(".$SqlExMethodExtensionName") + ".java"
        //获取包名
        val packageName = relativePath
            .windowsPathNormalize
            .substringBeforeLast('/')
            .removePrefix("/").removeSuffix("/")
            .replace('/', '.')
        //类名
        val className =
            relativePath
                .windowsPathNormalize
                .substringAfterLast('/')
                .removePrefix("/")
                .removeSuffix(".${SqlExMethodExtensionName}")
        //解析sqlm文件内容
        val parser =
            SqlExMethodLanguageParser(CommonTokenStream(SqlExMethodLanguageLexer(CharStreams.fromString(content))))
        val root = parser.root()
        val imports = root.importEx()
        val methods = root.method()
        //生成方法片段
        val methodSegments = methods
            .map { method ->
                //获取到方法名
                val methodName = method.methodName().text
                //获取到sql文本
                val sqlText = method.sql().text
                //解析sql
                val namedParameterSQL = sqlText.namedParameterSQL
                val fields = session.getFields(namedParameterSQL.sql)
                val inExprPositions = session.getInExprPositions(namedParameterSQL.sql)

                //TODO:假设都是select语句
                //结果类类名
                val resultClassName = method.returnType()?.text ?: "${methodName.pascalName}Result"
                //生成字段源码
                val resultFieldSegments = fields
                    .map {
                        //language=JAVA
                        """
                            private ${it.javaType} _${it.name};
                            public ${it.javaType} get${it.name.pascalName}() {
                                return this._${it.name};
                            }
                            public void set${it.name.pascalName}(${it.javaType} value) {
                                this._${it.name} = value;
                            }
                        """.trimIndent()
                    }
                //合成一个实体类源码
                //language=JAVA
                val resultClassContent = """
                    @SqlExGenerated
                    static class $resultClassName {
                        ${resultFieldSegments.joinToString("\n")}
                    }
                """.trimIndent()

                //SQL语句中的参数
                val parametersInSQL = namedParameterSQL.parameters.map { it.name }.distinct()
                //sqlm方法签名中的参数
                val parametersInMethod = method.paramList()?.param()?.map {
                    val paramType = if (it.paramRepeat() == null) {
                        it.paramType().text
                    } else {
                        "List<${it.paramType().text}>"
                    }
                    Pair(it.paramName().text, paramType)
                } ?: listOf()
                //检查sqlm方法签名中是否存在同名参数
                if (parametersInMethod.map { it.first }.distinct().size < parametersInMethod.size)
                    throw Exception("方法[$methodName]签名中存在同名参数")
                //检查SQL中参数和sqlm中参数的一致性
                val noExistInMethod = parametersInSQL.find { !parametersInMethod.map { p -> p.first }.contains(it) }
                if (noExistInMethod != null)
                    throw Exception("参数${noExistInMethod}在方法[$methodName]签名中未定义")
                //解析SQL参数在方法签名参数中的位置,做好映射
                val parameterPosition =
                    namedParameterSQL.parameters.map {
                        parametersInMethod.map { it.first }.indexOfFirst { p -> p == it.name }
                    }

                //方法片段
                """
                    $resultClassContent
        
                    @SqlExScript("${namedParameterSQL.sql.literal}")
                    @SqlExParameterPosition({${parameterPosition.joinToString()}})
                    @SqlExMarkerPosition({${namedParameterSQL.parameters.joinToString { it.position.toString() }}})
                    ${inExprPositions.joinToString("\n") { "@SqlExInExprPosition(not=${it.not},marker=${it.marker},start=${it.start},end=${it.end})" }}
                    List<$resultClassName> $methodName(${parametersInMethod.joinToString(", ") { "${it.second} ${it.first}" }});
                """.trimIndent()
            }
        //整个java文件内容
        //language=JAVA
        val javaFileContent = """
            package ${packageName};
            
            //core依赖
            import me.danwi.sqlex.core.annotation.*;
            //常用依赖
            import java.util.List;
            //数据类型依赖
            ${imports.joinToString("\n") { "import ${it.className().text};" }}
            
            @SqlExGenerated
            public interface $className {
                ${methodSegments.joinToString("\n")}
            }
        """.trimIndent()
        return JavaFile(className, packageName, javaRelativePath, javaFileContent)
    }

    fun close() {
        try {
            session.execute("drop database $databaseName")
        } finally {
            session.close()
        }
    }
}

class JavaFile(val className: String, val packageName: String, val relativePath: String, val source: String) {
    val javaClassQualifiedName = "$packageName.$className"
}

val Field.javaType: String
    get() {
        if (this.dbType == "bit") {//bit(n)
            return if (this.length == 1L) {
                "Boolean"
            } else {
                "byte[]"
            }
        } else if (this.dbType == "tinyint") {//tinyint(n) 或者 BOOL, BOOLEAN
            return if (this.length == 1L) {
                "Boolean"
            } else {
                "Integer"
            }
        }
        return when (this.dbType) {
            "smallint", "mediumint", "int", "integer" -> "Integer"
            "bigint" -> "Long"
            "float" -> "Float"
            "double" -> "Double"
            "decimal" -> "java.math.BigDecimal"
            "date" -> "java.sql.Date"
            "datetime", "timestamp" -> "java.sql.Timestamp"
            "time" -> "java.sql.Time"
            "year" -> "java.sql.Date"
            "char", "varchar" -> "String"
            "binary", "varbinary",
            "tinyblob", "blob", "mediumblob", "longblob" -> "byte[]"
            else -> "Object"
        }
    }
