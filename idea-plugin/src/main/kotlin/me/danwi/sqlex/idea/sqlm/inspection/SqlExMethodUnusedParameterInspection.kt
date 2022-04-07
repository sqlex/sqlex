package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemHighlightType
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElementVisitor
import com.intellij.psi.PsiFile
import me.danwi.sqlex.idea.sqlm.SqlExMethodFile
import me.danwi.sqlex.parser.util.namedParameterSQL

class SqlExMethodUnusedParameterInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitFile(file: PsiFile) {
                if (file !is SqlExMethodFile) return

                //获取到所有的方法定义
                val methods = file.root?.methods ?: listOf()

                //参数未使用检测
                for (method in methods) {
                    val params = method.paramList?.params?.mapNotNull { it.paramName } ?: continue
                    val usedParams = method.sql?.text?.namedParameterSQL?.parameters?.map { it.name } ?: listOf()
                    params.filter {
                        usedParams.none { u -> it.text == u }
                    }.forEach {
                        holder.registerProblem(
                            it,
                            "未使用方法签名中定义的参数 ${it.text}",
                            ProblemHighlightType.WEAK_WARNING
                        )
                    }
                }
            }
        }
    }
}