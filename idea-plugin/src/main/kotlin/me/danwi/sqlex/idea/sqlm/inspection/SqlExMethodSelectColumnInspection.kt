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
                    val fields = sqlSubtree.fields ?: return
                    //列名数量大于1才进行检查
                    if (fields.size > 1) {
                        //判断列名是否重复
                        val duplicateFieldNames =
                            fields.groupingBy { it.name.pascalName }.eachCount().filter { it.value > 1 }
                        if (duplicateFieldNames.isNotEmpty()) {
                            holder.registerProblem(
                                element,
                                textRange,
                                "存在重复的列名 ${duplicateFieldNames.map { "'${it.key}'" }.joinToString(", ")}"
                            )
                        }
                        //判断列名是否非法
                        val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
                        val invalidFieldNames = fields.map { it.name.pascalName }.filter { !regex.matches(it) }
                        if (invalidFieldNames.isNotEmpty()) {
                            holder.registerProblem(
                                element, textRange, "存在非法的列名 ${invalidFieldNames.joinToString(", ")}"
                            )
                        }
                    }
                } catch (_: Exception) {
                }
            }
        }
    }
}