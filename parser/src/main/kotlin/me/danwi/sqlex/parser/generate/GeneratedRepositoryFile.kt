package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.apt.SqlExAPTCheckedAnnotationProcessor
import me.danwi.sqlex.core.RepositoryLike
import me.danwi.sqlex.core.annotation.repository.*
import me.danwi.sqlex.parser.Repository
import javax.lang.model.element.Modifier

const val RepositoryClassName = "Repository"

class GeneratedRepositoryFile(
    rootPackage: String,
    private val converters: List<String>,
    private val schemas: List<String>,
    private val tableClassNames: List<String>,
    private val methodClassNames: List<String>,
    private val repository: Repository,
) : GeneratedJavaFile(rootPackage, RepositoryClassName) {
    override fun generate(): TypeSpec {
        val tableAnnotationSpecBuilder = AnnotationSpec.builder(SqlExTables::class.java)
        tableClassNames.forEach { tableAnnotationSpecBuilder.addMember("value", "\$L.class", it) }
        val methodAnnotationSpecBuilder = AnnotationSpec.builder(SqlExMethods::class.java)
        methodClassNames.forEach { methodAnnotationSpecBuilder.addMember("value", "\$L.class", it) }
        return TypeSpec.interfaceBuilder(RepositoryClassName)
            .addModifiers(Modifier.PUBLIC)
            .addSuperinterface(RepositoryLike::class.java)
            .addAnnotation(
                AnnotationSpec.builder(SqlExConverterCheck::class.java)
                    .build()
            )
            .addAnnotation(
                AnnotationSpec.builder(SqlExAPTChecked::class.java)
                    .addMember("value", "\$L.class", SqlExAPTCheckedAnnotationProcessor.CheckedStubClassName)
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
                    val sqls = repository.getSQLsOfScript(script)
                    val builder = AnnotationSpec
                        .builder(SqlExSchema::class.java)
                        .addMember("version", "\$L", version)
                    sqls.forEach { builder.addMember("scripts", "\$S", it) }
                    builder.build()
                }
            )
            .addAnnotation(tableAnnotationSpecBuilder.build())
            .addAnnotation(methodAnnotationSpecBuilder.build())
            .build()
    }
}