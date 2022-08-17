package me.danwi.sqlex.idea.sqlm

import com.intellij.codeInsight.daemon.impl.HighlightInfo
import com.intellij.codeInsight.daemon.impl.HighlightInfoType
import com.intellij.codeInsight.daemon.impl.HighlightVisitor
import com.intellij.codeInsight.daemon.impl.analysis.HighlightInfoHolder
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import me.danwi.sqlex.idea.sqlm.psi.*

class SqlExMethodSemanticHighlighter : HighlightVisitor {
    private var highlightInfoHolder: HighlightInfoHolder? = null

    override fun suitableForFile(file: PsiFile): Boolean {
        return file is SqlExMethodFile
    }

    override fun visit(element: PsiElement) {
        val color = getHighlighterColor(element) ?: return
        val highlightInfo = HighlightInfo.newHighlightInfo(HighlightInfoType.INFORMATION)
            .range(element.textRange)
            .textAttributes(color)
            .create()
        highlightInfoHolder?.add(highlightInfo)
    }

    private fun getHighlighterColor(element: PsiElement): TextAttributesKey? {
        return when (element) {
            is ClassNameSubtree -> DefaultLanguageHighlighterColors.CLASS_REFERENCE
            is ReturnTypeSubtree -> DefaultLanguageHighlighterColors.CLASS_REFERENCE
            is MethodNameSubtree -> DefaultLanguageHighlighterColors.INSTANCE_METHOD
            is ParamNameSubtree -> DefaultLanguageHighlighterColors.PARAMETER
            is ParamTypeSubtree -> DefaultLanguageHighlighterColors.STATIC_FIELD
            else -> null
        }
    }

    override fun analyze(
        file: PsiFile,
        updateWholeFile: Boolean,
        holder: HighlightInfoHolder,
        action: Runnable
    ): Boolean {
        this.highlightInfoHolder = holder
        action.run()
        return true
    }

    override fun clone(): HighlightVisitor {
        return SqlExMethodSemanticHighlighter()
    }
}