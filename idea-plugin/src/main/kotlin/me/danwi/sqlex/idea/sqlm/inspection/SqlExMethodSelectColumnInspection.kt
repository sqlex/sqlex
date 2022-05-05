package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree

class SqlExMethodSelectColumnInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //找到方法
                if (element !is MethodSubtree) return
                //获取sql元素
                val sqlSubtree = element.sql ?: return
                try {
                    //获取列名
                    val fields = sqlSubtree.fields ?: return
                    //判断列名是否重复
                    val duplicateFieldNames = fields.groupingBy { it.name }.eachCount().filter { it.value > 1 }
                    //判断列名是否非法
                    val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
                    val invalidFieldNames = fields.map { it.name }.filter { !regex.containsMatchIn(it) }

                    if (duplicateFieldNames.isNotEmpty()) {
                        holder.registerProblem(
                            sqlSubtree,
                            "存在重复的列名 ${duplicateFieldNames.map { "'${it.key}'" }.joinToString(", ")}"
                        )
                    } else if (invalidFieldNames.isNotEmpty()) {
                        holder.registerProblem(sqlSubtree, "存在非法的列名 ${invalidFieldNames.joinToString(", ")}")
                    }
                } catch (_: Exception) {
                }
            }
        }
    }
}