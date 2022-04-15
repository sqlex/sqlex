package me.danwi.sqlex.parser

import me.danwi.sqlex.parser.config.SqlExConfig
import me.danwi.sqlex.parser.config.createSqlExConfig
import me.danwi.sqlex.parser.exception.SqlExRepositoryException
import me.danwi.sqlex.parser.exception.SqlExRepositoryGenerateException
import me.danwi.sqlex.parser.exception.SqlExRepositoryMethodException
import me.danwi.sqlex.parser.exception.SqlExRepositorySchemaException
import me.danwi.sqlex.parser.util.*
import org.antlr.v4.runtime.CharStreams
import org.antlr.v4.runtime.CommonTokenStream
import java.io.File
import java.nio.file.Paths
import java.util.concurrent.atomic.AtomicInteger

//数据名称索引
private val index = AtomicInteger()

private const val RepositoryClassName = "Repository"

class RepositoryBuilder(private val config: SqlExConfig) {
    private val databaseName = "sqlex_${index.getAndIncrement()}"

    private val session: Session = Session(databaseName)

    private val rootPackageRelativePath =
        config.rootPackage?.packageNameToRelativePath ?: throw SqlExRepositoryException("无法获取根包信息")

    private var currentSchemaVersion = -1

    private val schemaContentCache = mutableListOf<String>()

    fun addSchema(relativePath: String, script: String) {
        try {
            //schema文件必须包含在根包下面
            if (!relativePath.startsWith(rootPackageRelativePath)) {
                throw SqlExRepositorySchemaException(
                    relativePath,
                    "Schema文件必须在根包下,当前根包: ${rootPackageRelativePath.relativePathToPackageName} ,Schema文件所在包: ${relativePath.relativePathToPackageName}"
                )
            }
            //获取schema的版本
            val version = relativePath.schemaFileVersion
            if (version != currentSchemaVersion + 1)
                if (currentSchemaVersion == -1)
                    throw SqlExRepositorySchemaException(relativePath, "Schema文件的版本必须从0开始")
                else
                    throw SqlExRepositorySchemaException(
                        relativePath,
                        "Schema文件版本必须连续且不重复,当前需要 ${currentSchemaVersion + 1} 版本,实际提供 ${version} 版本"
                    )
            //执行
            session.executeScript(script)
            //添加到缓存
            schemaContentCache.add(script)
            currentSchemaVersion++
        } catch (e: Exception) {
            try {
                session.execute("drop database $databaseName")
            } finally {
                session.close()
                throw SqlExRepositorySchemaException(relativePath, e.message)
            }
        }
    }

    fun build(): Repository {
        try {
            return Repository(databaseName, Session(databaseName), config, schemaContentCache)
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

class Repository(
    private val databaseName: String,
    private val session: Session,
    private val config: SqlExConfig,
    private val schemas: List<String>
) {
    private val rootPackage = config.rootPackage ?: throw SqlExRepositoryException("无法获取根包信息")
    private val converters = config.converters

    val DDL: String
        get() = session.DDL

    //SqlEx Repository顶级类
    val repositoryJavaFile: JavaFile by lazy {
        JavaFile(
            RepositoryClassName,
            rootPackage,
            "${rootPackage.packageNameToRelativePath}/${RepositoryClassName}.java",
            //language=JAVA
            """
                package $rootPackage;
                
                import me.danwi.sqlex.core.annotation.*;
                import me.danwi.sqlex.core.RepositoryLike;
                
                @SqlExConverterCheck
                @SqlExGenerated
                ${
                converters
                    .mapIndexed { index, converter -> "@SqlExConverter(order=${index},converter=${converter}.class)" }
                    .joinToString("\n")
            }
                ${
                schemas
                    .mapIndexed { version, script -> "@SqlExSchema(version=$version, script=\"${script.literal}\")" }
                    .joinToString("\n")
            }
                public interface $RepositoryClassName extends RepositoryLike {
                
                }
            """.trimIndent()
        )
    }

    //生成整个java文件
    fun generateJavaFile(relativePath: String, content: String): JavaFile {
        try {
            //java相对路径名
            val javaRelativePath = relativePath.sqlmPathToJavaPath
            //获取包名
            val packageName = javaRelativePath.relativePathToPackageName
            //判断是否根包的合法性
            if (!packageName.startsWith(rootPackage))
                throw SqlExRepositoryMethodException(
                    relativePath,
                    "Method文件必须在根包下,当前根包: $rootPackage ,Method文件所在包: $packageName"
                )
            //类名
            val className = javaRelativePath.classNameOfJavaRelativePath
            //解析sqlm文件内容
            val parser =
                SqlExMethodLanguageParser(CommonTokenStream(SqlExMethodLanguageLexer(CharStreams.fromString(content))))
            val root = parser.root()
            val imports = root.importEx()
            val methods = root.method()
            //import部分
            val importSourcePart = generateImport(imports)
            //方法部分
            val methodSourceParts = methods.map { generateMethod(it) }
            //整个java文件内容
            //language=JAVA
            val javaFileContent = """
                package ${packageName};
                
                //core依赖
                import me.danwi.sqlex.core.annotation.*;
                //常用依赖
                import java.util.List;
                //数据类型依赖
                $importSourcePart
                
                @SqlExGenerated
                public interface $className extends ${repositoryJavaFile.javaClassQualifiedName} {
                    ${methodSourceParts.joinToString("\n")}
                }
            """.trimIndent()
            return JavaFile(className, packageName, javaRelativePath, javaFileContent)
        } catch (e: Exception) {
            throw SqlExRepositoryMethodException(relativePath, e.message ?: "未知的Method解析错误")
        }
    }

    //生成import部分
    private fun generateImport(imports: List<SqlExMethodLanguageParser.ImportExContext>): String {
        return imports.joinToString("\n") { "import ${it.className().text};" }
    }

    //生成方法
    private fun generateMethod(method: SqlExMethodLanguageParser.MethodContext): String {
        //获取到sql文本
        val sqlText = method.sql().text
        //解析sql的命名参数信息
        val namedParameterSQL = sqlText.namedParameterSQL
        //获取到sql语句信息
        val statementInfo = session.getStatementInfo(namedParameterSQL.sql)
        //根据类型来生成不同到方法
        return when (statementInfo.type) {
            StatementType.Select -> generateSelectMethod(method, namedParameterSQL, statementInfo)
            StatementType.Insert -> generateInsertMethod(method, namedParameterSQL, statementInfo)
            StatementType.Update -> generateUpdateOrDelete(method, namedParameterSQL, statementInfo)
            StatementType.Delete -> generateUpdateOrDelete(method, namedParameterSQL, statementInfo)
            else -> throw Exception("不支持的语句类型,只支持(select/insert/update/delete)")
        }
    }

    //生成select方法,包括方法返回值内部类
    private fun generateSelectMethod(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): String {
        //获取到方法名
        val methodName = method.methodName().text
        //结果类源码
        val resultClassName = method.returnType()?.text ?: "${methodName.pascalName}Result"
        val resultClassSourcePart = generateResultClass(resultClassName, namedParameterSQL.sql)
        //生成注解部分
        val annotationPart = generateAnnotation(
            namedParameterSQL.sql,
            method.paramList(),
            namedParameterSQL.parameters,
            statementInfo.inExprPositions
        )
        //方法中的参数
        val parameterPart = generateParameter(methodName, method.paramList(), namedParameterSQL.parameters)
        //返回方法的内容
        return """
            $resultClassSourcePart
            
            $annotationPart
            List<$resultClassName> $methodName($parameterPart);
        """.trimIndent()
    }

    //生成Insert方法,返回值为lastInsertID TODO:目前只支持单个自增生成的值,以后改写为多个
    private fun generateInsertMethod(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): String {
        //获取到方法名
        val methodName = method.methodName().text
        //生成注解部分
        val annotationPart = generateAnnotation(
            namedParameterSQL.sql,
            method.paramList(),
            namedParameterSQL.parameters,
            statementInfo.inExprPositions
        )
        //方法中的参数
        val parameterPart = generateParameter(methodName, method.paramList(), namedParameterSQL.parameters)
        //返回方法的内容
        return """
            $annotationPart
            Long $methodName($parameterPart);
        """.trimIndent()
    }

    //生成Update/Delete方法,返回值为影响的行数
    private fun generateUpdateOrDelete(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): String {
        //获取到方法名
        val methodName = method.methodName().text
        //生成注解部分
        val annotationPart = generateAnnotation(
            namedParameterSQL.sql,
            method.paramList(),
            namedParameterSQL.parameters,
            statementInfo.inExprPositions
        )
        //方法中的参数
        val parameterPart = generateParameter(methodName, method.paramList(), namedParameterSQL.parameters)
        //返回方法的内容
        return """
            $annotationPart
            Long $methodName($parameterPart);
        """.trimIndent()
    }


    //生成参数签名部分
    private fun generateParameter(
        methodName: String,
        paramList: SqlExMethodLanguageParser.ParamListContext?,
        parameters: List<NamedParameter>
    ): String {
        //SQL语句中的参数
        val parametersInSQL = parameters.map { it.name }.distinct()
        //sqlm方法签名中的参数
        val parametersInMethod = paramList?.param()?.map {
            val paramType = if (it.paramRepeat() == null) {
                it.paramType().text
            } else {
                "List<${it.paramType().text}>"
            }
            Pair(it.paramName().text, paramType)
        } ?: listOf()
        //检查sqlm方法签名中是否存在同名参数
        if (parametersInMethod.map { it.first }.distinct().size < parametersInMethod.size)
            throw Exception("方法[${methodName}]签名中存在同名参数")
        //检查SQL中参数和sqm中参数的一致性
        val noExistInMethod = parametersInSQL.find { !parametersInMethod.map { p -> p.first }.contains(it) }
        if (noExistInMethod != null)
            throw Exception("参数${noExistInMethod}在方法[$methodName]签名中未定义")
        //返回参数部分源码
        return parametersInMethod.joinToString(", ") { "${it.second} ${it.first}" }
    }

    //生成方法上注解部分
    private fun generateAnnotation(
        sql: String,
        paramList: SqlExMethodLanguageParser.ParamListContext?,
        parameters: List<NamedParameter>,
        inExprPositions: Array<InExprPosition>
    ): String {
        //获取方法签名中的参数
        val parametersInMethod = paramList?.param()?.map { it.paramName().text } ?: listOf()
        //解析SQL参数在方法签名参数中的位置,做好映射
        val parameterPosition = parameters.map { parametersInMethod.indexOfFirst { p -> p == it.name } }
        //返回注解内容
        //language=JAVA
        return """
            @SqlExParameterCheck
            @SqlExScript("${sql.literal}")
            @SqlExParameterPosition({${parameterPosition.joinToString()}})
            @SqlExMarkerPosition({${parameters.joinToString { it.position.toString() }}})
            ${inExprPositions.joinToString("\n") { "@SqlExInExprPosition(not=${it.not},marker=${it.marker},start=${it.start},end=${it.end})" }}
        """.trimIndent()
    }

    //生成结果内部类
    private fun generateResultClass(className: String, sql: String): String {
        //获取到sql的返回字段
        val fields = session.getFields(sql)
        val fieldSourceParts = fields.map { generateField(it) }
        //返回结果类内容
        //language=JAVA
        return """
            @SqlExGenerated
            static class $className {
                ${fieldSourceParts.joinToString("\n")}
            }
        """.trimIndent()
    }

    //生成结果内部类的字段
    private fun generateField(field: Field): String {
        val javaType = getJavaType(field)
        //language=JAVA
        return """
            private $javaType _${field.name};
            public $javaType get${field.name.pascalName}() {
                return this._${field.name};
            }
            public void set${field.name.pascalName}($javaType value) {
                this._${field.name} = value;
            }
        """.trimIndent()
    }

    //获取字段的java类型名称
    private fun getJavaType(field: Field): String {
        if (field.dbType == "bit") { //bit(n)
            return if (field.length == 1L) "Boolean" else "byte[]"
        } else if (field.dbType == "tinyint") { //tinyint(n) 或者 bool, boolean
            //TODO: tinyInt1isBit为false时, Integer; 为true时, Boolean且size是1. 默认为false
            return "Integer"
        } else if (listOf("smallint", "mediumint").contains(field.dbType)) { //smallint, mediumint(不管是否unsigned)
            return "Integer"
        } else if (listOf("int", "integer").contains(field.dbType)) { //int, integer(unsigned时, java.lang.Long)
            return if (field.unsigned) "Long" else "Integer"
        } else if (field.dbType == "bigint") { //bigint(unsigned时, java.math.BigInteger)
            return if (field.unsigned) "Long" else "java.math.BigInteger"
        } else if (field.dbType == "float") { //float
            return "Float"
        } else if (field.dbType == "double") { //double
            return "Double"
        } else if (field.dbType == "decimal") { //decimal
            return "java.math.BigDecimal"
        } else if (field.dbType == "date") { //date
            return "java.sql.Date"
        } else if (field.dbType == "datetime") { //datetime
            return "java.sql.Timestamp"
        } else if (field.dbType == "timestamp") { //timestamp
            return "java.sql.Timestamp"
        } else if (field.dbType == "time") { //time
            return "java.sql.Time"
        } else if (field.dbType == "year") { //year
            return "java.sql.Date"
        } else if (listOf("char", "varchar").contains(field.dbType)) { //char, varchar
            return if (field.binary) "byte[]" else "String"
        } else if (listOf(
                "binary",
                "varbinary",
                "tinyblob",
                "blob",
                "mediumblob",
                "longblob"
            ).contains(field.dbType)
        ) { //binary, varbinary, tinyblob, blob, mediumblob, longblob
            return "byte[]"
        } else if (listOf(
                "tinytext",
                "text",
                "mediumtext",
                "longtext",
                "enum",
                "set"
            ).contains(field.dbType)
        ) { //tinytext, text, mediumtext, longtext
            return "String"
        } else {
            //return "Object"
            //内测阶段直接抛出异常, 便于排错
            throw Exception("${field.dbType} 映射失败!!!")
        }
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

//给定一个sqlex的source root,将代码生成到指定到文件中
fun generateRepositorySource(sourceRoot: File, outputDir: File) {
    //获取目录下的所有文件
    val files = sourceRoot.walk()
    //读取配置文件
    var tempConfigContent: String? = null
    files.maxDepth(1)
        .filter { it.isFile && it.absolutePath.isSqlExConfigFilePath }
        .forEach {
            if (tempConfigContent == null)
                tempConfigContent = it.readText()
            else
                throw SqlExRepositoryException("${sourceRoot.absolutePath}下有多个sqlex配置文件")
        }
    val config = createSqlExConfig(tempConfigContent ?: return)
    //获取到schema文件集合
    val schemaFiles = files
        .filter { it.isFile && it.name.isSqlExSchemaFilePath }
        .map { Pair(it.name.schemaFileVersion, it) }
        .filter { it.first != null }
        .sortedBy { it.first }
        .map { it.second }
    //构建repository
    val builder = RepositoryBuilder(config)
    schemaFiles.forEach { builder.addSchema(it.absolutePath.relativePathTo(sourceRoot.absolutePath), it.readText()) }
    val repository = builder.build()
    try {
        //在生成的源码目录下写入一个文件做标识
        val tagFile = File(outputDir.absolutePath, SqlExGeneratedTagFileName)
        if (!tagFile.exists()) {
            tagFile.parentFile.mkdirs()
            tagFile.writeText("this directory is auto generate by sqlex, DO NOT change anything in it")
        }
        //生成顶级Repository类文件
        val repositoryJavaFile = repository.repositoryJavaFile
        val repositorySourceFile = Paths.get(outputDir.absolutePath, repositoryJavaFile.relativePath).toFile()
        repositorySourceFile.parentFile.mkdirs()
        if (repositorySourceFile.exists())
            throw SqlExRepositoryGenerateException(repositorySourceFile.absolutePath, "重复源码生成")
        repositorySourceFile.writeText(repositoryJavaFile.source)
        //获取到所有的sqlm文件
        files
            .filter { it.isFile && it.absolutePath.isSqlExMethodFilePath }
            .map {
                repository.generateJavaFile(
                    it.absolutePath.relativePathTo(sourceRoot.absolutePath),
                    it.readText()
                )
            }
            .forEach {
                val sourceFile = Paths.get(outputDir.absolutePath, it.relativePath).toFile()
                sourceFile.parentFile.mkdirs()
                if (sourceFile.exists())
                    throw SqlExRepositoryGenerateException(sourceFile.absolutePath, "重复源码生成")
                sourceFile.writeText(it.source)
            }
    } finally {
        repository.close()
    }
}
