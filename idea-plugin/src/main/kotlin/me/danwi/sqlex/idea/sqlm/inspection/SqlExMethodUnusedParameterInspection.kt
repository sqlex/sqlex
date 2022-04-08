package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemHighlightType
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.sql.psi.SqlParameter
import me.danwi.sqlex.idea.sqlm.psi.MethodNameSubtree
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.visit

class SqlExMethodUnusedParameterInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (element !is MethodNameSubtree) return

                val method = element.parent
                if (method !is MethodSubtree) return

                //获取SQL中使用的参数
                val injectedSQLFile = method.sql?.injectedSQLFile ?: return
                val paramsInSQL = mutableListOf<String>()
                injectedSQLFile.visit {
                    if (it is SqlParameter) {
                        val nameElement = it.nameElement ?: return@visit
                        paramsInSQL.add(nameElement.text)
                    }
                }

                //获取方法签名中的参数列表
                val paramsInMethod = method.paramList?.params?.mapNotNull { it.paramName } ?: return
                paramsInMethod.forEach { paramInMethod ->
                    if (paramsInSQL.none { paramInMethod.text == it }) {
                        holder.registerProblem(
                            paramInMethod,
                            "该参数未在SQL中使用",
                            ProblemHighlightType.WEAK_WARNING
                        )
                    }
                }
            }
        }
    }
}