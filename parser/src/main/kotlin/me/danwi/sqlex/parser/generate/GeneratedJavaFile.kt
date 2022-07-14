package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.JavaFile
import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.core.annotation.SqlExGenerated
import me.danwi.sqlex.parser.exception.SqlExRepositoryMethodException
import me.danwi.sqlex.parser.util.packageNameToRelativePath

//让javapoet不要省略package
const val DoNotHidePackagePrefix = "sqlex.no.not.hide.package."

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
            return@lazy JavaFile.builder(packageName, type).indent("    ").build()
                .toString().replace(DoNotHidePackagePrefix, "")
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