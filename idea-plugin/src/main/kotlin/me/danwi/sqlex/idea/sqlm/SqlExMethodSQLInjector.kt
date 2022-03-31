package me.danwi.sqlex.idea.sqlm

import com.intellij.lang.injection.MultiHostInjector
import com.intellij.lang.injection.MultiHostRegistrar
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.sql.psi.SqlLanguage
import me.danwi.sqlex.idea.sqlm.psi.SqlSubtree

class SqlExMethodSQLInjector : MultiHostInjector {
    override fun elementsToInjectIn(): MutableList<out Class<out PsiElement>> {
        return mutableListOf(SqlSubtree::class.java)
    }

    override fun getLanguagesToInject(registrar: MultiHostRegistrar, context: PsiElement) {
        if (context is SqlSubtree) {
            registrar.startInjecting(SqlLanguage.INSTANCE)
                .addPlace(null, null, context, TextRange.from(0, context.textLength))
                .doneInjecting()
        }
    }
}
