package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.*
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.common.Paged
import me.danwi.sqlex.core.annotation.*
import me.danwi.sqlex.core.type.PagedResult
import me.danwi.sqlex.parser.*
import me.danwi.sqlex.parser.exception.SqlExRepositoryMethodException
import me.danwi.sqlex.parser.util.*
import org.antlr.v4.runtime.CharStreams
import org.antlr.v4.runtime.CommonTokenStream
import javax.lang.model.element.Modifier

private class ImportedClasses(imported: List<SqlExMethodLanguageParser.ImportExContext>) {
    val imported = imported.map {
        val qualifiedName = it.className().text
        ClassName.get(qualifiedName.substringBeforeLast("."), qualifiedName.substringAfterLast("."))
    }

    fun getClassName(className: String): ClassName {
        return imported.find { it.simpleName() == className } ?: ClassName.get("", className)
    }
}

class GeneratedMethodFile(
    private val rootPackage: String,
    relativePath: String,
    private val content: String,
    private val session: Session
) :
    GeneratedJavaFile(
        relativePath.sqlmPathToJavaPath.relativePathToPackageName,
        relativePath.sqlmPathToJavaPath.classNameOfJavaRelativePath
    ) {

    //文件中导入的类
    private var importedClasses: ImportedClasses

    //在生成过程中产生到内部类
    private val innerClasses = mutableListOf<TypeSpec>()

    init {
        try {
            //判断是否根包的合法性
            if (packageName != rootPackage && !packageName.startsWith("$rootPackage."))
                throw Exception("Method文件必须在根包下,当前根包: $rootPackage ,Method文件所在包: $packageName")

            //解析sqlm文件内容
            val parser =
                SqlExMethodLanguageParser(CommonTokenStream(SqlExMethodLanguageLexer(CharStreams.fromString(content))))
            val root = parser.root()
            val imports = root.importEx()

            //获取到import的类
            importedClasses = ImportedClasses(imports)
        } catch (e: Exception) {
            throw SqlExRepositoryMethodException(
                relativePath,
                e.message ?: "未知的Method解析错误(${e::class.java.simpleName})"
            )
        }
    }

    override fun generate(): TypeSpec {
        //解析sqlm文件内容
        val parser =
            SqlExMethodLanguageParser(CommonTokenStream(SqlExMethodLanguageLexer(CharStreams.fromString(content))))
        val root = parser.root()
        //获取到声明的方法
        val methods = root.method()
        //开始生成方法
        val methodSpecs = methods.map { generateMethod(it) }
        //返回生成的接口
        return TypeSpec
            .interfaceBuilder(className)
            .addAnnotation(
                //所属的repository
                AnnotationSpec
                    .builder(SqlExRepository::class.java)
                    .addMember("value", "\$T.class", ClassName.get(rootPackage, RepositoryClassName))
                    .build()
            )
            .addModifiers(Modifier.PUBLIC)
            .addTypes(innerClasses)
            .addMethods(methodSpecs)
            .build()
    }

    //生成方法
    private fun generateMethod(method: SqlExMethodLanguageParser.MethodContext): MethodSpec {
        //获取到SQL文本
        val sqlText = method.sql().text
        //解析SQL的命名参数信息
        val namedParameterSQL = sqlText.namedParameterSQL
        //获取到SQL语句信息
        val statementInfo = session.getStatementInfo(namedParameterSQL.sql)
        //只有select中才能使用paged
        if (method.paged() != null && statementInfo.type != StatementType.Select)
            throw Exception("只有Select方法才能标记为分页方法")
        //根据类型来生成不同的方法
        return when (statementInfo.type) {
            StatementType.Select -> generateSelectMethod(method, namedParameterSQL, statementInfo)
            StatementType.Insert -> generateInsertMethod(method, namedParameterSQL, statementInfo)
            StatementType.Update -> generateUpdateMethod(method, namedParameterSQL, statementInfo)
            StatementType.Delete -> generateDeleteMethod(method, namedParameterSQL, statementInfo)
            else -> throw Exception("不支持的语句类型,只支持(select/insert/update/delete)")
        }
    }

    private fun generateSelectMethod(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): MethodSpec {
        //获取到方法名
        val methodName = method.methodName().text
        //构建方法
        val methodSpec = MethodSpec.methodBuilder(methodName)
            .addModifiers(Modifier.PUBLIC, Modifier.ABSTRACT)
            .addAnnotation(SqlExSelect::class.java)
        //是否为分页方法
        val isPaged = method.paged() != null
        //生成注解
        methodSpec.addAnnotations(
            generateAnnotation(
                method.paramList(),
                namedParameterSQL,
                statementInfo
            )
        )
        //生成参数
        methodSpec.addParameters(generateParameter(methodName, method.paramList(), namedParameterSQL, isPaged))
        //返回值
        //生成结果类
        val resultClassName = method.returnType()?.text ?: "${methodName.pascalName}Result"
        val resultClassSpec = generateResultClass(resultClassName, namedParameterSQL.sql)
        innerClasses.add(resultClassSpec)
        val resultTypeName = ClassName.get(packageName, className, resultClassName)
        var returnTypeName: TypeName = ParameterizedTypeName.get(ClassName.get(List::class.java), resultTypeName)
        //是否为单行查询
        if (statementInfo.hasLimit && statementInfo.limitRows.toInt() == 1) {
            //单行,判断是否有paged
            if (isPaged)
                throw Exception("单行查询不能声明为分页方法")
            returnTypeName = resultTypeName
            methodSpec.addAnnotation(SqlExOneRow::class.java)
        }
        //是否为分页
        if (isPaged) {
            returnTypeName = ParameterizedTypeName.get(ClassName.get(PagedResult::class.java), resultTypeName)
            methodSpec.addAnnotation(SqlExPaged::class.java)
        }
        //添加返回值
        methodSpec.returns(returnTypeName)
        return methodSpec.build()
    }

    private fun generateInsertMethod(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): MethodSpec {
        //获取到方法名
        val methodName = method.methodName().text
        //构建方法
        val methodSpec = MethodSpec.methodBuilder(methodName)
            .addModifiers(Modifier.PUBLIC, Modifier.ABSTRACT)
            .addAnnotation(SqlExInsert::class.java)
        //生成注解
        methodSpec.addAnnotations(
            generateAnnotation(
                method.paramList(),
                namedParameterSQL,
                statementInfo
            )
        )
        //生成参数
        methodSpec.addParameters(generateParameter(methodName, method.paramList(), namedParameterSQL, false))
        //添加返回值
        methodSpec.returns(ClassName.LONG)
        return methodSpec.build()
    }

    private fun generateUpdateMethod(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): MethodSpec {
        //获取到方法名
        val methodName = method.methodName().text
        //构建方法
        val methodSpec = MethodSpec.methodBuilder(methodName)
            .addModifiers(Modifier.PUBLIC, Modifier.ABSTRACT)
            .addAnnotation(SqlExUpdate::class.java)
        //生成注解
        methodSpec.addAnnotations(
            generateAnnotation(
                method.paramList(),
                namedParameterSQL,
                statementInfo
            )
        )
        //生成参数
        methodSpec.addParameters(generateParameter(methodName, method.paramList(), namedParameterSQL, false))
        //添加返回值
        methodSpec.returns(ClassName.LONG)
        return methodSpec.build()
    }

    private fun generateDeleteMethod(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): MethodSpec {
        //获取到方法名
        val methodName = method.methodName().text
        //构建方法
        val methodSpec = MethodSpec.methodBuilder(methodName)
            .addModifiers(Modifier.PUBLIC, Modifier.ABSTRACT)
            .addAnnotation(SqlExDelete::class.java)
        //生成注解
        methodSpec.addAnnotations(
            generateAnnotation(
                method.paramList(),
                namedParameterSQL,
                statementInfo
            )
        )
        //生成参数
        methodSpec.addParameters(generateParameter(methodName, method.paramList(), namedParameterSQL, false))
        //添加返回值
        methodSpec.returns(ClassName.LONG)
        return methodSpec.build()
    }

    private fun generateResultClass(resultClassName: String, sql: String): TypeSpec {
        val typeSpecBuilder = TypeSpec.classBuilder(resultClassName)
            .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
        //获取字段
        val fields = session.getFields(sql)
        //判断列名是否重复
        val duplicateFieldNames = fields.groupingBy { it.name }.eachCount().filter { it.value > 1 }
        if (duplicateFieldNames.isNotEmpty()) {
            throw Exception("重复的列名 ${duplicateFieldNames.map { "'${it.key}'" }.joinToString(", ")}")
        }
        //判断列名是否非法
        val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
        val invalidFieldNames = fields.map { it.name }.filter { !regex.containsMatchIn(it) }
        if (invalidFieldNames.isNotEmpty()) {
            throw Exception("非法的列名 ${invalidFieldNames.joinToString(", ")}")
        }
        //给实体添加getter/setter
        session.getFields(sql)
            .map { Pair(it.name, getJavaType(it)) }
            .forEach { typeSpecBuilder.addGetterAndSetter(it.first, it.second) }
        return typeSpecBuilder.build()
    }

    private fun generateAnnotation(
        paramList: SqlExMethodLanguageParser.ParamListContext?,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo
    ): List<AnnotationSpec> {
        //获取方法签名中的参数
        val parametersInMethod = paramList?.param()?.map { it.paramName().text } ?: listOf()
        //解析SQL参数在方法签名参数中的位置,做好映射
        val parameterPosition =
            namedParameterSQL.parameters.map { parametersInMethod.indexOfFirst { p -> p == it.name } }
        //返回的注解集合
        val annotationSpecs = mutableListOf<AnnotationSpec>()
        //参数检查
        annotationSpecs.add(
            AnnotationSpec
                .builder(SqlExParameterCheck::class.java)
                .build()
        )
        //SQL脚本
        annotationSpecs.add(
            AnnotationSpec
                .builder(SqlExScript::class.java)
                .addMember("value", "\$S", namedParameterSQL.sql)
                .build()
        )
        //SQL参数在方法参数列表中的索引
        val parameterPositionAnnotationSpec = AnnotationSpec
            .builder(SqlExParameterPosition::class.java)
        parameterPosition.forEach { parameterPositionAnnotationSpec.addMember("value", "\$L", it) }
        annotationSpecs.add(parameterPositionAnnotationSpec.build())
        //SQL参数?在SQL字符串中的索引
        val markerPositionAnnotationSpec = AnnotationSpec
            .builder(SqlExMarkerPosition::class.java)
        namedParameterSQL.parameters.forEach { markerPositionAnnotationSpec.addMember("value", "\$L", it.position) }
        annotationSpecs.add(markerPositionAnnotationSpec.build())
        //in (?)的位置
        annotationSpecs.addAll(
            statementInfo.inExprPositions
                .map {
                    AnnotationSpec
                        .builder(SqlExInExprPosition::class.java)
                        .addMember("not", "\$L", it.not)
                        .addMember("marker", "\$L", it.marker)
                        .addMember("start", "\$L", it.start)
                        .addMember("end", "\$L", it.end)
                        .build()
                }
        )
        return annotationSpecs
    }

    private fun generateParameter(
        methodName: String,
        paramList: SqlExMethodLanguageParser.ParamListContext?,
        namedParameterSQL: NamedParameterSQL,
        paged: Boolean
    ): List<ParameterSpec> {
        //SQL语句中到参数
        val parametersInSQL = namedParameterSQL.parameters.map { it.name }.distinct()
        //sqlm方法签名中到参数
        val parametersInMethod = paramList?.param()?.map {
            val paramType = if (it.paramRepeat() == null) {
                importedClasses.getClassName(it.paramType().text)
            } else {
                ParameterizedTypeName.get(
                    ClassName.get(List::class.java),
                    importedClasses.getClassName(it.paramType().text)
                )
            }
            Pair(it.paramName().text, paramType)
        }?.toMutableList() ?: mutableListOf()
        //检查SQL中参数和sqm中参数的一致性
        val noExistInMethod = parametersInSQL.find { !parametersInMethod.map { p -> p.first }.contains(it) }
        if (noExistInMethod != null)
            throw Exception("参数${noExistInMethod}在方法[$methodName]签名中未定义")
        //如果是分页方法,还需要补上两个分页参数
        if (paged) {
            parametersInMethod.add(Pair(Paged.PageSizeParameterName, ClassName.LONG))
            parametersInMethod.add(Pair(Paged.PageNoParameterName, ClassName.LONG))
        }
        //检查sqlm方法签名中是否存在同名参数
        if (parametersInMethod.map { it.first }.distinct().size < parametersInMethod.size)
            throw Exception("方法[${methodName}]签名中存在同名参数")
        //返回参数
        return parametersInMethod.map { ParameterSpec.builder(it.second, it.first).build() }
    }

    private fun getJavaType(field: Field): TypeName {
        if (field.dbType == "bit") { //bit(n)
            return if (field.length == 1L) ClassName.bestGuess("Boolean") else ArrayTypeName.of(ClassName.BYTE)
        } else if (field.dbType == "tinyint") { //tinyint(n) 或者 bool, boolean
            //TODO: tinyInt1isBit为false时, Integer; 为true时, Boolean且size是1. 默认为false
            return ClassName.bestGuess("Integer")
        } else if (listOf("smallint", "mediumint").contains(field.dbType)) { //smallint, mediumint(不管是否unsigned)
            return ClassName.bestGuess("Integer")
        } else if (listOf("int", "integer").contains(field.dbType)) { //int, integer(unsigned时, java.lang.Long)
            return if (field.unsigned) ClassName.bestGuess("Long") else ClassName.bestGuess("Integer")
        } else if (field.dbType == "bigint") { //bigint(unsigned时, java.math.BigInteger)
            return if (field.unsigned) ClassName.bestGuess("Long") else ClassName.bestGuess("java.math.BigInteger")
        } else if (field.dbType == "float") { //float
            return ClassName.bestGuess("Float")
        } else if (field.dbType == "double") { //double
            return ClassName.bestGuess("Double")
        } else if (field.dbType == "decimal") { //decimal
            return ClassName.get(java.math.BigDecimal::class.java)
        } else if (field.dbType == "date") { //date
            return ClassName.get(java.time.LocalDate::class.java)
        } else if (field.dbType == "datetime") { //datetime
            return ClassName.get(java.time.LocalDateTime::class.java)
        } else if (field.dbType == "timestamp") { //timestamp
            return ClassName.get(java.time.LocalDateTime::class.java)
        } else if (field.dbType == "time") { //time
            return ClassName.get(java.time.LocalTime::class.java)
        } else if (field.dbType == "year") { //year
            return ClassName.get(java.time.LocalDate::class.java)
        } else if (listOf("char", "varchar").contains(field.dbType)) { //char, varchar
            return if (field.binary) ArrayTypeName.of(ClassName.BYTE) else ClassName.bestGuess("String")
        } else if (listOf(
                "binary",
                "varbinary",
                "tinyblob",
                "blob",
                "mediumblob",
                "longblob"
            ).contains(field.dbType)
        ) { //binary, varbinary, tinyblob, blob, mediumblob, longblob
            return ArrayTypeName.of(ClassName.BYTE)
        } else if (listOf(
                "tinytext",
                "text",
                "mediumtext",
                "longtext",
                "enum",
                "set"
            ).contains(field.dbType)
        ) { //tinytext, text, mediumtext, longtext
            return ClassName.bestGuess("String")
        } else {
            //return "Object"
            //内测阶段直接抛出异常, 便于排错
            throw Exception("${field.dbType} 映射失败!!!")
        }
    }
}
