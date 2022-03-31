package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.sql.psi.SqlParameter
import me.danwi.sqlex.idea.util.extension.containingSqlExMethod

class SqlExMethodUnnamedParameterInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //只处理sql参数
                if (element !is SqlParameter)
                    return
                //判断是不是一个 ? 参数,是不是属于sqlm文件的注入SQL
                if (element.text == "?" && element.containingSqlExMethod != null) {
                    holder.registerProblem(element, "SqlEx Method中不允许使用非命名的预处理参数")
                }
            }
        }
    }
}