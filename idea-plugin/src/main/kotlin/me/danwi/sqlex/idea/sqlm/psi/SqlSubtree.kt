package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.lang.injection.InjectedLanguageManager
import com.intellij.psi.ElementManipulators
import com.intellij.psi.LiteralTextEscaper
import com.intellij.psi.PsiLanguageInjectionHost
import com.intellij.psi.tree.IElementType
import com.intellij.sql.psi.SqlFile
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class SqlSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType),
    PsiLanguageInjectionHost {
    override fun isValidHost(): Boolean {
        return true
    }

    override fun updateText(text: String): PsiLanguageInjectionHost {
        return ElementManipulators.handleContentChange(this, text)
    }

    override fun createLiteralTextEscaper(): LiteralTextEscaper<out PsiLanguageInjectionHost> {
        return LiteralTextEscaper.createSimple(this)
    }

    val injectedSQLFile: SqlFile?
        get() {
            val files = mutableListOf<SqlFile>()
            InjectedLanguageManager.getInstance(project)
                .enumerate(this) { file, _ ->
                    if (file is SqlFile)
                        files.add(file)
                }
            return files.firstOrNull()
        }
}