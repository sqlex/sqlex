package me.danwi.sqlex.idea.sqlm.kotlin

import com.intellij.codeInsight.navigation.actions.GotoDeclarationHandler
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiMethod
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.childrenOf
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
            return targetElement.containingClass
                ?.sqlexMethodFile?.psiFile
                ?.childrenOf<MethodSubtree>()
                ?.filter { it.methodName?.methodName == targetElement.name }
                ?.toTypedArray()
        } else if (targetElement is PsiClass) {
            val sqlmFile = targetElement.sqlexMethodFile?.psiFile ?: return null
            return arrayOf(sqlmFile)
        }
        return null
    }
}