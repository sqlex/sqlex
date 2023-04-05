package me.danwi.sqlex.idea.sqlm.kotlin

import com.intellij.psi.PsiElement
import com.intellij.psi.PsiMethod
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.methodSubtree
import me.danwi.sqlex.idea.util.extension.parentOf
import org.jetbrains.kotlin.idea.references.mainReference
import org.jetbrains.kotlin.psi.KtNameReferenceExpression
import me.danwi.sqlex.idea.sqlm.SqlExMethodDocumentationProvider as SqlExMethodJavaDocumentationProvider

class SqlExMethodDocumentationProvider : SqlExMethodJavaDocumentationProvider() {
    override fun findMethodSubtree(contextElement: PsiElement): MethodSubtree? {
        //解析引用元素
        val referenceElement = contextElement.parentOf<KtNameReferenceExpression>() ?: return null
        val targetElement = referenceElement.mainReference.resolve()
        if (targetElement !is PsiMethod) return null

        //找到对应的方法
        return targetElement.methodSubtree
    }
}