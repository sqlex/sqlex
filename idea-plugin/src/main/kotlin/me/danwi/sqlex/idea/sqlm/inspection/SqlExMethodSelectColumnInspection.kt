package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import me.danwi.sqlex.parser.StatementType

class SqlExMethodSelectColumnInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //找到方法
                if (element !is MethodSubtree) return
                //获取sql元素
                val sqlSubtree = element.sql ?: return
                //获取sql文本
                val sql = sqlSubtree.text ?: return
                //获取session
                val session = element.containingFile.virtualFile.sqlexRepositoryService?.repository?.session ?: return
                //判断是否是select
                if (session.getStatementInfo(sql).type != StatementType.Select) return
                //获取列名
                val fields = session.getFields(sql)
                //判断列名是否重复
                val duplicateFieldNames = fields.groupingBy { it.name }.eachCount().filter { it.value > 1 }
                if (duplicateFieldNames.isNotEmpty()) {
                    holder.registerProblem(
                        sqlSubtree,
                        "存在重复的列名 ${duplicateFieldNames.map { "'${it.key}'" }.joinToString(", ")}"
                    )
                }
                //判断列名是否非法
                val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
                val invalidFieldNames = fields.map { it.name }.filter { !regex.containsMatchIn(it) }
                if (invalidFieldNames.isNotEmpty()) {
                    holder.registerProblem(sqlSubtree, "存在非法的列名 ${invalidFieldNames.joinToString(", ")}")
                }
            }
        }
    }
}