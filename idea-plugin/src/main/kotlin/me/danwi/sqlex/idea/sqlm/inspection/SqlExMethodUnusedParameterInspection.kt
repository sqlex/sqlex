package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.*
import com.intellij.openapi.project.Project
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.psi.util.elementType
import com.intellij.sql.psi.SqlParameter
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_COMMA
import me.danwi.sqlex.idea.sqlm.psi.MethodNameSubtree
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.childrenOf

class SqlExMethodUnusedParameterInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (element !is MethodNameSubtree) return

                val method = element.parent
                if (method !is MethodSubtree) return

                //获取SQL中使用的参数
                val injectedSQLFile = method.sql?.injectedSQLFile ?: return
                val paramsInSQL = injectedSQLFile.childrenOf<SqlParameter>().mapNotNull { it.nameElement?.text }

                //获取方法签名中的参数列表
                val paramsInMethod = method.paramList?.params?.mapNotNull { it.paramName } ?: return
                paramsInMethod.forEach { paramInMethod ->
                    if (paramsInSQL.none { paramInMethod.text == it }) {
                        holder.registerProblem(
                            paramInMethod,
                            "该参数未在SQL中使用",
                            ProblemHighlightType.WEAK_WARNING,
                            object : LocalQuickFix {
                                override fun getFamilyName(): String {
                                    return "删除 ${paramInMethod.text} 参数"
                                }

                                override fun applyFix(project: Project, descriptor: ProblemDescriptor) {
                                    val paramName = descriptor.psiElement
                                    val param = paramName.parent
                                    val prevComma = param.prevSibling
                                    val nextComma = param.nextSibling

                                    param.delete()
                                    if (prevComma.elementType == TOKEN_COMMA) {
                                        prevComma.delete()
                                    } else if (nextComma.elementType == TOKEN_COMMA) {
                                        nextComma.delete()
                                    }
                                }
                            }
                        )
                    }
                }
            }
        }
    }
}