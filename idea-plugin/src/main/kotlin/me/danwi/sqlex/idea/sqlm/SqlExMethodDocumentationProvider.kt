package me.danwi.sqlex.idea.sqlm

import com.intellij.lang.documentation.DocumentationProvider
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.richcopy.HtmlSyntaxInfoUtil
import com.intellij.openapi.util.Key
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiJavaCodeReferenceElement
import com.intellij.psi.PsiMethod
import com.intellij.sql.psi.SqlLanguage
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.parentOf
import me.danwi.sqlex.idea.util.extension.psiFile
import me.danwi.sqlex.idea.util.extension.sqlexMethodFile

private val sqlCacheKey = Key<String>("me.danwi.sqlex.document.SqlCacheKey")

open class SqlExMethodDocumentationProvider : DocumentationProvider {
    open fun findMethodSubtree(contextElement: PsiElement): MethodSubtree? {
        //解析引用元素
        val referenceElement = contextElement.parentOf<PsiJavaCodeReferenceElement>() ?: return null
        val resolveResult = referenceElement.multiResolve(false)
        //找到对应的方法
        return resolveResult
            .map { it.element }
            .filterIsInstance<PsiMethod>()
            .firstNotNullOfOrNull {
                it.containingClass
                    ?.sqlexMethodFile
                    ?.psiFile
                    ?.childrenOf<MethodSubtree>()
                    ?.find { m -> m.methodName?.methodName == it.name }
            }
    }


    override fun getCustomDocumentationElement(
        editor: Editor,
        file: PsiFile,
        contextElement: PsiElement?,
        targetOffset: Int
    ): PsiElement? {
        if (contextElement == null)
            return null
        //找到对应的MethodSubtree
        val methodSubtree = findMethodSubtree(contextElement) ?: return null
        //找到对应的sql语句
        val sqlScript = methodSubtree.sql?.text ?: return null
        //把sql放入到user data中
        contextElement.putUserData(sqlCacheKey, sqlScript.trimIndent().trim())
        return contextElement
    }

    override fun generateDoc(element: PsiElement?, originalElement: PsiElement?): String? {
        val sql = element?.getUserData(sqlCacheKey) ?: return null
        val stringBuilder = StringBuilder()

        HtmlSyntaxInfoUtil.appendStyledSpan(
            stringBuilder,
            DefaultLanguageHighlighterColors.LINE_COMMENT,
            "SQL:",
            1.0.toFloat()
        )

        stringBuilder.append("<br/>")

        HtmlSyntaxInfoUtil.appendHighlightedByLexerAndEncodedAsHtmlCodeSnippet(
            stringBuilder,
            element.project,
            SqlLanguage.INSTANCE,
            sql,
            true,
            1.0.toFloat()
        )
        return stringBuilder.toString()
    }
}