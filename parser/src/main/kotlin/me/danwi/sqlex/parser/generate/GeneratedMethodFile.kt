package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.*
import me.danwi.sqlex.common.Paged
import me.danwi.sqlex.core.annotation.SqlExDataAccessObject
import me.danwi.sqlex.core.annotation.SqlExRepository
import me.danwi.sqlex.core.annotation.method.*
import me.danwi.sqlex.core.annotation.method.parameter.*
import me.danwi.sqlex.core.annotation.method.type.SqlExDelete
import me.danwi.sqlex.core.annotation.method.type.SqlExInsert
import me.danwi.sqlex.core.annotation.method.type.SqlExSelect
import me.danwi.sqlex.core.annotation.method.type.SqlExUpdate
import me.danwi.sqlex.core.type.PagedResult
import me.danwi.sqlex.parser.*
import me.danwi.sqlex.parser.exception.SqlExRepositoryMethodException
import me.danwi.sqlex.parser.util.*
import org.antlr.v4.runtime.CharStreams
import org.antlr.v4.runtime.CommonTokenStream
import org.jetbrains.annotations.Nullable
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
            .addAnnotation(SqlExDataAccessObject::class.java)
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
        //获取计划信息
        val planInfo = session.getPlanInfo(namedParameterSQL.sql)
        //只有select中才能使用paged
        if (method.paged() != null && statementInfo.type != StatementType.Select)
            throw Exception("只有Select方法才能标记为分页方法")
        //根据类型来生成不同的方法
        return when (statementInfo.type) {
            StatementType.Select -> generateSelectMethod(method, namedParameterSQL, statementInfo, planInfo)
            StatementType.Insert -> generateInsertMethod(method, namedParameterSQL, session, statementInfo, planInfo)
            StatementType.Update -> generateUpdateMethod(method, namedParameterSQL, statementInfo)
            StatementType.Delete -> generateDeleteMethod(method, namedParameterSQL, statementInfo)
            else -> throw Exception("不支持的语句类型,只支持(select/insert/update/delete)")
        }
    }

    private fun generateSelectMethod(
        method: SqlExMethodLanguageParser.MethodContext,
        namedParameterSQL: NamedParameterSQL,
        statementInfo: StatementInfo,
        planInfo: PlanInfo
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
        //获取字段
        val fields = planInfo.fields
        //判断是否为单列返回值
        val resultTypeName = if (fields.size == 1) {
            //如果是单列则将这一列对应的java类型作为结果类型
            methodSpec.addAnnotation(SqlExOneColumn::class.java)
            fields[0].JavaType
        } else {
            //如果是多列,则生成对应的实体类
            val resultClassName = method.returnType()?.text ?: "${methodName.pascalName}Result"
            val resultClassSpec =
                fields.toEntityClass(resultClassName, isStatic = true, constructor = false, nullableAnnotation = false)
            //把实体类添加到内部类
            innerClasses.add(resultClassSpec)
            ClassName.get(packageName, className, resultClassName)
        }
        //默认为一个结果类型的集合
        var returnTypeName: TypeName = ParameterizedTypeName.get(ClassName.get(List::class.java), resultTypeName)
        //是否为单行查询
        if (planInfo.maxOneRow) {
            //单行,判断是否有paged
            if (isPaged)
                throw Exception("单行查询不能声明为分页方法")
            //如果是单行,返回值直接就是结果类型,且可能为空
            returnTypeName = resultTypeName
            methodSpec.addAnnotation(SqlExOneRow::class.java)
            methodSpec.addAnnotation(Nullable::class.java)
        }
        //是否为分页
        if (isPaged) {
            //如果是分页,则用分页类型包装起来
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
        session: Session,
        statementInfo: StatementInfo,
        planInfo: PlanInfo
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
        //获取自动生成列的信息
        val autoIncrementColumn = session.getTableInfo(planInfo.insertTable).columns.find { it.isAutoIncrement }
        if (autoIncrementColumn == null)
            methodSpec.returns(ClassName.VOID)
        else
            methodSpec
                .addAnnotation(Nullable::class.java)
                .returns(autoIncrementColumn.JavaType)
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
        //? is null的位置
        annotationSpecs.addAll(
            statementInfo.isNullExprPositions
                .map {
                    AnnotationSpec
                        .builder(SqlExIsNullExprPosition::class.java)
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
}