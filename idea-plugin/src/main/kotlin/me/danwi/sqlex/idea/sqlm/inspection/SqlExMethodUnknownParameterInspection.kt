package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.sql.psi.SqlParameter
import me.danwi.sqlex.idea.util.extension.containingSqlExMethod

class SqlExMethodUnknownParameterInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //只处理sql参数
                if (element !is SqlParameter)
                    return
                //获取参数的名字
                val nameElement = element.nameElement ?: return
                //获取名称
                val name = nameElement.text
                //获取sqlex方法
                val method = element.containingSqlExMethod ?: return
                //获取已经定义的参数
                val paramNames = method.paramList?.params?.mapNotNull { it.paramName?.text?.trim() } ?: listOf()
                if (!paramNames.contains(name)) {
                    holder.registerProblem(nameElement, "方法签名中没有定义${name}参数")
                }
            }
        }
    }
}