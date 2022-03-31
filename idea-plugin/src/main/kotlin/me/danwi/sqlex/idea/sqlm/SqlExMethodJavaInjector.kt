package me.danwi.sqlex.idea.sqlm

import com.intellij.lang.injection.MultiHostInjector
import com.intellij.lang.injection.MultiHostRegistrar
import com.intellij.lang.java.JavaLanguage
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import me.danwi.sqlex.idea.sqlm.psi.ClassNameSubtree
import me.danwi.sqlex.idea.sqlm.psi.ParamTypeSubtree

class SqlExMethodJavaInjector : MultiHostInjector {
    override fun elementsToInjectIn(): MutableList<out Class<out PsiElement>> {
        return mutableListOf(ParamTypeSubtree::class.java, ClassNameSubtree::class.java)
    }

    override fun getLanguagesToInject(registrar: MultiHostRegistrar, context: PsiElement) {
        val file = context.containingFile
        if (file is SqlExMethodFile) {
            val imports = file.root?.imports ?: listOf()
            val importPart = imports.mapNotNull { it.className }.joinToString("\n") { "import ${it};" };

            if (context is ParamTypeSubtree) {
                registrar.startInjecting(JavaLanguage.INSTANCE)
                    .addPlace(
                        "$importPart\nclass Dummy { void dummy( ",
                        " dummy){} }",
                        context,
                        TextRange.from(0, context.textLength)
                    )
                    .doneInjecting()
            }
            if (context is ClassNameSubtree) {
                registrar.startInjecting(JavaLanguage.INSTANCE)
                    .addPlace("$importPart \n import ", ";", context, TextRange.from(0, context.textLength))
                    .doneInjecting()
            }
        }
    }
}