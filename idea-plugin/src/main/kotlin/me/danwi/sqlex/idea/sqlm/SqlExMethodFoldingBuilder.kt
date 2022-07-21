package me.danwi.sqlex.idea.sqlm

import com.intellij.lang.ASTNode
import com.intellij.lang.folding.FoldingBuilderEx
import com.intellij.lang.folding.FoldingDescriptor
import com.intellij.openapi.editor.Document
import com.intellij.psi.PsiElement
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree

class SqlExMethodFoldingBuilder : FoldingBuilderEx() {
    override fun buildFoldRegions(root: PsiElement, document: Document, quick: Boolean): Array<FoldingDescriptor> {
        if (root !is SqlExMethodFile)
            return arrayOf()
        //取得当前sqlm文件内所有的method
        val methodSubtrees = root.root?.methods ?: return arrayOf()
        return methodSubtrees
            .filter { it.textRange.length > 0 }
            .map { FoldingDescriptor(it, it.textRange) }
            .toTypedArray()
    }

    override fun getPlaceholderText(node: ASTNode): String? {
        val psiElement = node.psi
        if (psiElement !is MethodSubtree) return null
        return psiElement.presentation.presentableText
    }

    override fun isCollapsedByDefault(node: ASTNode): Boolean {
        return false
    }
}