package me.danwi.sqlex.idea.sqlm.kotlin

import com.intellij.psi.PsiElement
import com.intellij.psi.PsiMethod
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.parentOf
import me.danwi.sqlex.idea.util.extension.psiFile
import me.danwi.sqlex.idea.util.extension.sqlexMethodFile
import org.jetbrains.kotlin.nj2k.postProcessing.resolve
import org.jetbrains.kotlin.psi.KtNameReferenceExpression
import me.danwi.sqlex.idea.sqlm.SqlExMethodDocumentationProvider as SqlExMethodJavaDocumentationProvider


class SqlExMethodDocumentationProvider : SqlExMethodJavaDocumentationProvider() {
    override fun findMethodSubtree(contextElement: PsiElement): MethodSubtree? {
        //解析引用元素
        val referenceElement = contextElement.parentOf<KtNameReferenceExpression>() ?: return null
        val targetElement = referenceElement.resolve()
        if (targetElement !is PsiMethod) return null

        //找到对应的方法
        return targetElement.containingClass
            ?.sqlexMethodFile
            ?.psiFile
            ?.childrenOf<MethodSubtree>()
            ?.find { m -> m.methodName?.methodName == targetElement.name }
    }
}