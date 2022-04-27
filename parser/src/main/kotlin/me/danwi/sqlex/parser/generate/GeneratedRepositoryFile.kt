package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.TypeSpec
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
            .addSuperinterface(CorePackageName.getClassName("RepositoryLike"))
            .addAnnotation(
                AnnotationSpec.builder(CoreAnnotationsPackageName.getClassName("SqlExConverterCheck"))
                    .build()
            )
            .addAnnotations(
                converters.mapIndexed { order, converter ->
                    AnnotationSpec
                        .builder(CoreAnnotationsPackageName.getClassName("SqlExConverter"))
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
                        .builder(CoreAnnotationsPackageName.getClassName("SqlExSchema"))
                        .addMember("version", "\$L", version)
                    sqls.forEach { builder.addMember("scripts", "\$S", it) }
                    builder.build()
                }
            )
            .build()
    }
}