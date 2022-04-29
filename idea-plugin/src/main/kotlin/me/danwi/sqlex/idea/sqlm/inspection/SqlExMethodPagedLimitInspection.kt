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
                //获取分页方法
                element.containingSqlExMethod?.childrenOf<PagedSubtree>()?.firstOrNull() ?: return

                //判断是否有offset
                val hasOffset = element.children.any { it.text.toLowerCase() == "offset" }
                //判断是否有,
                val hasComma = element.children.any { it.text == "," }
                //获取数字常量
                val nums = element.children.filterIsInstance<SqlLiteralExpression>().map { it.text }
                //limit count文本
                val countText = if (hasOffset && nums.size == 2) { //存在offset且数字个数为2
                    nums[0]
                } else if (hasComma && nums.size == 2) { //存在,且数字个数为2
                    nums[1]
                } else if (nums.size == 1) { //数字个数为1
                    nums[0]
                } else { //其他情况, 直接返回
                    return
                }
                //判断是否是limit 1
                if (countText == "1") {
                    holder.registerProblem(element, "使用分页方法时不能使用Limit 1")
                } else {
                    holder.registerProblem(element, "正在分页方法内使用Limit N", ProblemHighlightType.WEAK_WARNING)
                }
            }
        }
    }
}