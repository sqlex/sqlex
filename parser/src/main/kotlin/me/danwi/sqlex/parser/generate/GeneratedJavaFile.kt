package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.ClassName
import com.squareup.javapoet.JavaFile
import com.squareup.javapoet.MethodSpec
import com.squareup.javapoet.TypeName
import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.core.annotation.SqlExGenerated
import me.danwi.sqlex.parser.exception.SqlExRepositoryMethodException
import me.danwi.sqlex.parser.util.packageNameToRelativePath
import me.danwi.sqlex.parser.util.pascalName
import javax.lang.model.element.Modifier

class PackageName(private val packageName: String) {
    fun getClassName(className: String): ClassName {
        return ClassName.get(packageName, className)
    }
}

abstract class GeneratedJavaFile(val packageName: String, val className: String) {
    val qualifiedName: String
        get() = "$packageName.$className"

    val relativePath: String
        get() = "${packageName.packageNameToRelativePath}/${className}.java"

    //生成的源码
    val source: String by lazy {
        try {
            val type = this.generate().toBuilder().addAnnotation(
                AnnotationSpec
                    .builder(SqlExGenerated::class.java)
                    .build()
            ).build()
            return@lazy JavaFile.builder(packageName, type).indent("    ").build().toString()
        } catch (e: Exception) {
            throw SqlExRepositoryMethodException(
                relativePath,
                e.message ?: "未知的Method解析错误(${e::class.java.simpleName})"
            )
        }
    }

    //具体的生成方法
    protected abstract fun generate(): TypeSpec
}

//给类添加getter/setter
fun TypeSpec.Builder.addGetterAndSetter(name: String, type: TypeName): TypeSpec.Builder {
    this.addField(type, "_${name}", Modifier.PRIVATE)
    this.addMethod(
        MethodSpec.methodBuilder("get${name.pascalName}")
            .addModifiers(Modifier.PUBLIC)
            .addStatement("return this._${name}")
            .returns(type)
            .build()
    )
    this.addMethod(
        MethodSpec.methodBuilder("set${name.pascalName}")
            .addModifiers(Modifier.PUBLIC)
            .addParameter(type, "value")
            .addStatement("this._${name} = value")
            .build()
    )
    return this
}