package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.*
import com.intellij.psi.tree.IElementType
import me.danwi.sqlex.idea.sqlm.SqlExMethodFile
import me.danwi.sqlex.idea.util.extension.sourceRootRelativePath
import me.danwi.sqlex.parser.util.relativePathToPackageName
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class ClassNameSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType),
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

    //获取参数对应的java类型
    val javaType: PsiClass?
        get() {
            return JavaPsiFacade.getInstance(project).findClass(this.text.trim(), this.resolveScope)
        }
}