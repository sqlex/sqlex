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
import me.danwi.sqlex.parser.PlanInfo
import me.danwi.sqlex.parser.StatementInfo
import me.danwi.sqlex.parser.StatementType
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

private val planInfoCacheKey = Key<PlanInfo>("me.danwi.sqlex.idea.sqlm.PlanInfoCacheKey")
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

    val planInfo: PlanInfo?
        get() {
            try {
                if (statementInfo?.type != StatementType.Select) return null
                val session = this.containingFile.virtualFile.sqlexRepositoryService?.repository?.session ?: return null
                val sql = this.text ?: return null
                return this.getUserData(planInfoCacheKey) ?: session.getPlanInfo(sql)
            } catch (_: Exception) {
                return null
            }
        }

    val statementInfo: StatementInfo?
        get() {
            try {
                val session = this.containingFile.virtualFile.sqlexRepositoryService?.repository?.session ?: return null
                val sql = this.text ?: return null
                return this.getUserData(statementInfoCacheKey) ?: session.getStatementInfo(sql)
            } catch (_: Exception) {
                return null
            }
        }

    override fun subtreeChanged() {
        this.putUserData(planInfoCacheKey, null)
        this.putUserData(statementInfoCacheKey, null)
    }
}