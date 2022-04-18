package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemHighlightType
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.sql.psi.SqlLimitClause
import com.intellij.sql.psi.SqlLiteralExpression
import com.intellij.sql.psi.SqlParenthesizedExpression
import me.danwi.sqlex.idea.sqlm.psi.PagedSubtree
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.containingSqlExMethod
import me.danwi.sqlex.idea.util.extension.parentOf

class SqlExMethodPagedLimitInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //获取limit
                if (element !is SqlLimitClause) return
                //判断是否在子句中
                if (element.parentOf<SqlParenthesizedExpression>() != null) return
                //判断是否有offset
                val hasOffset = element.children.any { it.text.lowercase() == "offset" }
                //判断是否是limit 1
                val count1 = if (hasOffset) element.childrenOf<SqlLiteralExpression>().firstOrNull()?.text == "1"
                else element.childrenOf<SqlLiteralExpression>().lastOrNull()?.text == "1"
                //获取分页方法
                element.containingSqlExMethod?.childrenOf<PagedSubtree>()?.firstOrNull() ?: return

                if (count1) {
                    holder.registerProblem(element, "使用分页方法时不能使用Limit 1")
                } else {
                    holder.registerProblem(element, "正在分页方法内使用Limit N", ProblemHighlightType.WEAK_WARNING)
                }
            }
        }
    }
}