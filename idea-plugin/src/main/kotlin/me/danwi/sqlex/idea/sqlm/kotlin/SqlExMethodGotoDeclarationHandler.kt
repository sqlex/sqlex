package me.danwi.sqlex.idea.sqlm.kotlin

import com.intellij.codeInsight.navigation.actions.GotoDeclarationHandler
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiMethod
import me.danwi.sqlex.idea.util.extension.methodSubtree
import me.danwi.sqlex.idea.util.extension.parentOf
import me.danwi.sqlex.idea.util.extension.psiFile
import me.danwi.sqlex.idea.util.extension.sqlexMethodFile
import org.jetbrains.kotlin.nj2k.postProcessing.resolve
import org.jetbrains.kotlin.psi.KtNameReferenceExpression

class SqlExMethodGotoDeclarationHandler : GotoDeclarationHandler {
    override fun getGotoDeclarationTargets(
        sourceElement: PsiElement?,
        offset: Int,
        editor: Editor?
    ): Array<PsiElement>? {
        if (sourceElement == null)
            return null

        //解析引用元素
        val referenceElement = sourceElement.parentOf<KtNameReferenceExpression>() ?: return null
        val targetElement = referenceElement.resolve()

        //判断类型
        if (targetElement is PsiMethod) {
            return arrayOf(targetElement.methodSubtree ?: return null)
        } else if (targetElement is PsiClass) {
            return arrayOf(targetElement.sqlexMethodFile?.psiFile ?: return null)
        }
        return null
    }
}