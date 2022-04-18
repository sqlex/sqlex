package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import me.danwi.sqlex.common.Paged.PageNoParameterName
import me.danwi.sqlex.common.Paged.PageSizeParameterName
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.sqlm.psi.PagedSubtree

class SqlExMethodPagedParameterInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //处理分页
                if (element !is PagedSubtree) return
                //处理方法名
                val methodSubtree = element.parent
                if (methodSubtree !is MethodSubtree) return
                //获取方法参数列表
                val paramsInMethod = methodSubtree.paramList?.params?.mapNotNull { it.paramName } ?: return

                paramsInMethod.forEach { paramInMethod ->
                    if (listOf(PageSizeParameterName, PageNoParameterName).contains(paramInMethod.text)) {
                        holder.registerProblem(paramInMethod, "使用分页方法时不能使用该参数名称")
                    }
                }
            }
        }
    }
}