package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.TypeSpec
import javax.lang.model.element.Modifier

const val RepositoryClassName = "Repository"

class GeneratedRepositoryFile(
    rootPackage: String,
    private val converters: List<String>,
    private val schemas: List<String>
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
                    AnnotationSpec
                        .builder(CoreAnnotationsPackageName.getClassName("SqlExSchema"))
                        .addMember("version", "\$L", version)
                        .addMember("script", "\$S", script)
                        .build()
                }
            )
            .build()
    }
}