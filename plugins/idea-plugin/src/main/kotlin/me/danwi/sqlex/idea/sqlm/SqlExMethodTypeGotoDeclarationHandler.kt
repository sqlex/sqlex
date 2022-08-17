package me.danwi.sqlex.idea.sqlm

import com.intellij.codeInsight.navigation.actions.GotoDeclarationHandler
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiElement
import me.danwi.sqlex.idea.sqlm.psi.ClassNameSubtree
import me.danwi.sqlex.idea.sqlm.psi.ParamTypeSubtree
import me.danwi.sqlex.idea.util.extension.parentOf

class SqlExMethodTypeGotoDeclarationHandler : GotoDeclarationHandler {
    override fun getGotoDeclarationTargets(
        sourceElement: PsiElement?,
        offset: Int,
        editor: Editor?
    ): Array<PsiElement>? {
        val paramTypeSubtree = sourceElement?.parentOf<ParamTypeSubtree>()
        if (paramTypeSubtree != null) {
            val psiClass = paramTypeSubtree.javaType ?: return null
            return arrayOf(psiClass)
        }

        val classNameSubtree = sourceElement?.parentOf<ClassNameSubtree>()
        if (classNameSubtree != null) {
            val psiClass = classNameSubtree.javaType ?: return null
            return arrayOf(psiClass)
        }

        return null
    }
}