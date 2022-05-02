package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.sql.psi.*
import com.intellij.sql.psi.impl.SqlReferenceExpressionImpl
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.idea.util.extension.childrenOf

class SqlExMethodSelectColumnInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (element !is SqlSelectClause) return
                //函数调用, 建议重命名
                element
                    .children
                    .filterIsInstance<SqlFunctionCallExpression>()
                    .map { holder.registerProblem(it, "请使用 AS 重命名") }
                //多表 *, 建议指定列
                if (element.parent.childrenOf<SqlJoinExpression>().isNotEmpty()) {
                    element.children.filterIsInstance<SqlReferenceExpressionImpl>().filter { it.text.endsWith(".*") }
                        .map { holder.registerProblem(it, "多表查询请明确指定列") }
                }
                val duplicate = HashMap<String, PsiElement>()
                val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
                element.children.forEach {
                    if (it is SqlReferenceExpressionImpl || it is SqlAsExpression) {
                        it.children.filterIsInstance<SqlIdentifier>().firstOrNull()?.let { e ->
                            //列名重复
                            if (null == duplicate[e.text]) duplicate[e.text] = it
                            else holder.registerProblem(it, "列名重复")
                            //列名非法
                            if (regex.containsMatchIn(it.text)) {
                                holder.registerProblem(it, "非法的列名")
                            }
                        }
                    }
                }
            }
        }
    }
}