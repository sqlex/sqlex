package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.lang.injection.InjectedLanguageManager
import com.intellij.openapi.util.Key
import com.intellij.psi.ElementManipulators
import com.intellij.psi.LiteralTextEscaper
import com.intellij.psi.PsiLanguageInjectionHost
import com.intellij.psi.tree.IElementType
import com.intellij.sql.psi.SqlFile
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import me.danwi.sqlex.parser.Field
import me.danwi.sqlex.parser.StatementInfo
import me.danwi.sqlex.parser.StatementType
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

private val fieldsCacheKey = Key<Array<Field>>("me.danwi.sqlex.idea.sqlm.FieldsCacheKey")
private val statementInfoCacheKey = Key<StatementInfo>("me.danwi.sqlex.idea.sqlm.StatementInfoCacheKey")

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

    val fields: Array<Field>?
        get() {
            if (statementInfo?.type != StatementType.Select) return null
            val session = this.containingFile.virtualFile.sqlexRepositoryService?.repository?.session ?: return null
            val sql = this.text ?: return null
            return this.getUserData(fieldsCacheKey) ?: session.getFields(sql)
        }

    val statementInfo: StatementInfo?
        get() {
            val session = this.containingFile.virtualFile.sqlexRepositoryService?.repository?.session ?: return null
            val sql = this.text ?: return null
            return this.getUserData(statementInfoCacheKey) ?: session.getStatementInfo(sql)
        }

    override fun subtreeChanged() {
        this.putUserData(fieldsCacheKey, null)
        this.putUserData(statementInfoCacheKey, null)
    }
}