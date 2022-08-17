package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.database.dialects.base.endOffset
import com.intellij.database.dialects.base.startOffset
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.sql.psi.SqlParenthesizedExpression
import com.intellij.sql.psi.SqlSelectClause
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.idea.util.extension.containingSqlExMethod
import me.danwi.sqlex.idea.util.extension.parentOf
import me.danwi.sqlex.parser.util.pascalName

class SqlExMethodSelectColumnInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (element !is SqlSelectClause) return
                //判断是否在子句中
                if (element.parentOf<SqlParenthesizedExpression>() != null) return
                //获取方法
                val methodSubtree = element.containingSqlExMethod ?: return
                //获取sql文本
                val sqlSubtree = methodSubtree.sql ?: return
                try {
                    //计算column偏移量
                    val expressions = element.expressions
                    if (expressions.isEmpty())
                        return
                    val startOffset = expressions.first().startOffsetInParent
                    val endOffset = expressions.last().endOffset - element.startOffset
                    if (endOffset < startOffset)
                        return
                    val textRange = TextRange(startOffset, endOffset)
                    //获取列名
                    val fields = sqlSubtree.planInfo?.fields ?: return
                    //列名数量小于2, 直接返回
                    if (fields.size < 2) return
                    //判断列名是否重复
                    val duplicateFieldNames =
                        fields.groupBy(keySelector = { it.name.pascalName }, valueTransform = { it.name })
                            .filter { it.value.size > 1 }.values
                    if (duplicateFieldNames.isNotEmpty()) {
                        holder.registerProblem(
                            element,
                            textRange,
                            "存在重复的列名 ${duplicateFieldNames.joinToString(", ")}"
                        )
                    }
                    //判断列名是否非法
                    val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
                    val invalidFieldNames = fields.filter { !regex.matches(it.name.pascalName) }
                    if (invalidFieldNames.isNotEmpty()) {
                        holder.registerProblem(
                            element, textRange, "存在非法的列名 ${invalidFieldNames.joinToString(", ") { it.name }}"
                        )
                    }
                } catch (_: Exception) {
                }
            }
        }
    }
}