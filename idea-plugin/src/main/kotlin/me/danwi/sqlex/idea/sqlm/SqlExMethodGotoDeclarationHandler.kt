package me.danwi.sqlex.idea.sqlm

import com.intellij.codeInsight.navigation.actions.GotoDeclarationHandler
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiJavaCodeReferenceElement
import com.intellij.psi.PsiMethod
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.parentOf
import me.danwi.sqlex.idea.util.extension.psiFile
import me.danwi.sqlex.idea.util.extension.sqlexMethodFile

class SqlExMethodGotoDeclarationHandler : GotoDeclarationHandler {
    override fun getGotoDeclarationTargets(
        sourceElement: PsiElement?,
        offset: Int,
        editor: Editor?
    ): Array<PsiElement>? {
        if (sourceElement == null)
            return null

        //解析引用元素
        val referenceElement = sourceElement.parentOf<PsiJavaCodeReferenceElement>() ?: return null
        val resolveResult = referenceElement.multiResolve(false)

        //处理方法
        val methodTargets = resolveResult
            .map { it.element }
            .filterIsInstance<PsiMethod>()
            .mapNotNull {
                val psiClass = it.containingClass ?: return@mapNotNull null
                val psiFile = psiClass.sqlexMethodFile?.psiFile ?: return@mapNotNull null
                return@mapNotNull psiFile.childrenOf<MethodSubtree>()
                    .find { m -> m.methodName?.methodName == it.name }
            }

        //处理类
        val classTargets = resolveResult
            .map { it.element }
            .filterIsInstance<PsiClass>()
            .mapNotNull {
                return@mapNotNull it.sqlexMethodFile?.psiFile
            }

        //返回目标结果
        return (methodTargets + classTargets).toTypedArray()
    }
}